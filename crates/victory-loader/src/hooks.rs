//! Opt-in compatibility hooks for known title builds.
//!
//! Only per-build offsets and short byte signatures live here (asset policy:
//! nothing of the title itself). Every hook checks the build's hash and the
//! exact bytes it replaces, and does nothing otherwise.

use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
use windows_sys::Win32::System::Memory::{PAGE_EXECUTE_READWRITE, VirtualProtect};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// First 16 hex digits of the BLAKE3 of the Series X 1.0.2.1 executable.
const BUILD_1_0_2_1: &str = "b99f876b74d4a77f";

/// Environment variable that turns the watchdog hook on.
pub(crate) const NO_WATCHDOG_ENV: &str = "VICTORY_NO_WATCHDOG";

/// One code patch: `rva` must hold exactly `expect` before `new` is written.
struct Patch {
    what: &'static str,
    rva: usize,
    expect: &'static [u8],
    new: &'static [u8],
}

/// The title's watchdog thread ends the process on purpose when its main
/// thread has not advanced for about 30 one-second ticks. Two places do it:
/// the handler it calls (`mov dword ptr [0], imm32`) and, when the handler
/// returns or is unset, a fallback right after the call (`mov dword ptr [0],
/// 0; int3`). Jumping over the whole block returns the loop to its normal
/// path; the first patch makes the handler harmless if something else calls it.
const WATCHDOG: &[Patch] = &[
    Patch {
        what: "watchdog: skip the call and the fallback fault",
        rva: 0x2215c27,
        // mov rax, [rip + disp32] (the handler pointer)
        expect: &[0x48, 0x8B, 0x05, 0x1A, 0x07, 0xFA, 0x04],
        // jmp +0x18 (to the loop's normal continuation), then NOPs
        new: &[0xEB, 0x18, 0x90, 0x90, 0x90, 0x90, 0x90],
    },
    Patch {
        what: "watchdog: handler's deliberate fault",
        rva: 0x23cf683,
        expect: &[0xC7, 0x04, 0x25, 0, 0, 0, 0, 0x21, 0xD4, 0x8B, 0x4D],
        // an eleven-byte NOP
        new: &[0x66, 0x66, 0x2E, 0x0F, 0x1F, 0x84, 0, 0, 0, 0, 0],
    },
];

/// Applies the watchdog patches whose bytes match. Returns how many applied.
///
/// # Safety
/// `base` is the mapped image whose original BLAKE3 is `hash`, and no title
/// code is running yet.
pub(crate) unsafe fn disarm_watchdog(base: usize, hash: &blake3::Hash) -> usize {
    if !hash.to_hex().starts_with(BUILD_1_0_2_1) {
        return 0;
    }
    WATCHDOG
        .iter()
        // SAFETY: forwarded from this function's contract.
        .filter(|p| unsafe { apply(base, p) })
        .count()
}

/// # Safety
/// See `disarm_watchdog`.
unsafe fn apply(base: usize, p: &Patch) -> bool {
    let at = (base + p.rva) as *mut u8;
    // SAFETY: the offset is inside the mapped .text of this exact build.
    let now = unsafe { std::slice::from_raw_parts(at, p.expect.len()) };
    if now != p.expect || p.new.len() != p.expect.len() {
        tracing::warn!(what = p.what, "hook skipped: the bytes are not the expected ones");
        return false;
    }
    let mut old = 0;
    // SAFETY: changing the protection of those bytes only, restored below.
    if unsafe { VirtualProtect(at as _, p.new.len(), PAGE_EXECUTE_READWRITE, &mut old) } == 0 {
        return false;
    }
    // SAFETY: writable now; the bytes are the ones checked above.
    unsafe {
        std::ptr::copy_nonoverlapping(p.new.as_ptr(), at, p.new.len());
        VirtualProtect(at as _, p.new.len(), old, &mut old);
        FlushInstructionCache(GetCurrentProcess(), at as _, p.new.len());
    }
    tracing::info!(what = p.what, "hook applied");
    true
}

#[cfg(test)]
mod tests {
    use super::{BUILD_1_0_2_1, WATCHDOG};

    /// A patch whose replacement is not the same length as the bytes it
    /// checks is skipped at run time with only a warning; catch it here.
    #[test]
    fn every_patch_replaces_what_it_checks() {
        for p in WATCHDOG {
            assert_eq!(
                p.new.len(),
                p.expect.len(),
                "{}: {} new bytes for {} original",
                p.what,
                p.new.len(),
                p.expect.len()
            );
            assert!(!p.expect.is_empty(), "{}: no bytes to check", p.what);
            assert!(!p.what.is_empty());
        }
    }

    /// Two patches at one offset would be a copy-paste slip.
    #[test]
    fn the_patches_do_not_overlap() {
        for (i, a) in WATCHDOG.iter().enumerate() {
            for b in &WATCHDOG[i + 1..] {
                let (a0, a1) = (a.rva, a.rva + a.expect.len());
                let (b0, b1) = (b.rva, b.rva + b.expect.len());
                assert!(a1 <= b0 || b1 <= a0, "{} overlaps {}", a.what, b.what);
            }
        }
    }

    #[test]
    fn the_build_identity_is_sixteen_hex_digits() {
        assert_eq!(BUILD_1_0_2_1.len(), 16);
        assert!(BUILD_1_0_2_1.chars().all(|c| c.is_ascii_hexdigit()));
    }
}

/// Environment variable that turns the handle table's report on.
pub(crate) const HANDLE_TABLE_ENV: &str = "VICTORY_HANDLE_TABLE";

/// The title's table of its pools' object handles (build 1.0.2.1): the
/// count of its groups at `rva`, a pointer to the groups after it. A group
/// is 16 control bytes (a slot's 7 hash bits; 0x80 and up: no key) and 16
/// slots of 8 bytes, a slot's 32-bit key first. A key's home group is its
/// hash (`murmur3`'s finish) shifted right by 7, under the count less one;
/// a lookup walks group by group from there.
///
/// Under Proton a third to a half of the CPU's samples in play were that
/// walk (kc/foxhound run 52, `perf`, the title's main threads): how full
/// the table is and how far its keys are from home tell why.
const HANDLE_TABLE_RVA: usize = 0x876fc48;

fn murmur_finish(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^ (h >> 16)
}

/// Of the groups' bytes `groups` (the count's worth): slots used, slots
/// with no key, and the keys' distances from their home groups (mean,
/// longest).
fn handle_table_shape(groups: &[u8], count: usize) -> (usize, usize, f64, usize) {
    let (mut used, mut free, mut sum, mut longest) = (0usize, 0usize, 0usize, 0usize);
    for g in 0..count {
        let group = &groups[g * 144..(g + 1) * 144];
        for slot in 0..16 {
            if group[slot] >= 0x80 {
                free += 1;
                continue;
            }
            used += 1;
            let at = 16 + slot * 8;
            let key = u32::from_le_bytes([group[at], group[at + 1], group[at + 2], group[at + 3]]);
            let home = (murmur_finish(key) >> 7) as usize & (count - 1);
            let distance = g.wrapping_sub(home) & (count - 1);
            sum += distance;
            longest = longest.max(distance);
        }
    }
    (used, free, if used > 0 { sum as f64 / used as f64 } else { 0.0 }, longest)
}

/// Every ten seconds, how full the title's handle table is and how far its
/// keys are from home (`VICTORY_HANDLE_TABLE=1`, build 1.0.2.1 only). Reads
/// through `ReadProcessMemory`: a bad pointer is a failed read, not a fault.
///
/// # Safety
/// `base` is the mapped image whose original BLAKE3 is `hash`.
pub(crate) unsafe fn report_handle_table(base: usize, hash: &blake3::Hash) {
    use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    if !hash.to_hex().starts_with(BUILD_1_0_2_1) {
        return;
    }
    let read = |at: usize, into: &mut [u8]| -> bool {
        let mut got = 0;
        // SAFETY: our own process; the system checks the range.
        unsafe { ReadProcessMemory(GetCurrentProcess(), at as *const _, into.as_mut_ptr().cast(), into.len(), &mut got) != 0 && got == into.len() }
    };
    let _ = std::thread::Builder::new().name("victory-handle-table".into()).spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));
            let mut head = [0u8; 16];
            if !read(base + HANDLE_TABLE_RVA, &mut head) {
                continue;
            }
            let count = u32::from_le_bytes(head[0..4].try_into().unwrap()) as usize;
            let groups_at = u64::from_le_bytes(head[8..16].try_into().unwrap()) as usize;
            if count == 0 || !count.is_power_of_two() || count > 1 << 22 || groups_at == 0 {
                tracing::info!(count, groups_at = format_args!("{groups_at:#x}"), "the title's handle table: not made yet");
                continue;
            }
            let mut groups = vec![0u8; count * 144];
            if !read(groups_at, &mut groups) {
                continue;
            }
            let (used, free, mean, longest) = handle_table_shape(&groups, count);
            tracing::info!(
                slots = count * 16,
                used,
                full_percent = format_args!("{:.1}", 100.0 * used as f64 / (count * 16) as f64),
                free,
                mean_groups_from_home = format_args!("{mean:.1}"),
                longest_groups_from_home = longest,
                "the title's handle table ({HANDLE_TABLE_ENV})"
            );
        }
    });
}

#[cfg(test)]
mod handle_table_tests {
    use super::*;

    /// A key in its home group is 0 groups away; one put two groups on is
    /// 2; empty slots count as free.
    #[test]
    fn keys_are_measured_from_home() {
        let count = 8;
        let mut groups = vec![0x80u8; count * 144];
        let key = 0x1234_5678u32;
        let home = (murmur_finish(key) >> 7) as usize & (count - 1);
        let put = |groups: &mut Vec<u8>, g: usize, slot: usize, key: u32| {
            groups[g * 144 + slot] = 0x11;
            groups[g * 144 + 16 + slot * 8..g * 144 + 20 + slot * 8].copy_from_slice(&key.to_le_bytes());
        };
        put(&mut groups, home, 0, key);
        put(&mut groups, (home + 2) % count, 3, key);
        let (used, free, mean, longest) = handle_table_shape(&groups, count);
        assert_eq!((used, free, longest), (2, count * 16 - 2, 2));
        assert!((mean - 1.0).abs() < 1e-9);
    }
}
