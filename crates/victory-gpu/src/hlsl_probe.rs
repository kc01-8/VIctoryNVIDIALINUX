//! The HLSL helpers the runtime compiles, each compiled the way its call site
//! does: same source name, entry point, target and flags, with the system's
//! `D3DCompile`. It needs no device. On Windows this guards the sources. Under
//! Wine or Proton it shows which helpers the builtin `d3dcompiler_47` refuses
//! (see docs/linux-proton.md). Run it with `--nocapture` to see each line.
//!
//! Not listed: the opt-in `VICTORY_INPLACE_PROBE` shader, and the depth
//! plane's, which are private to `host.rs`.

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::core::{PCSTR, s};

struct Helper {
    what: &'static str,
    /// Where the runtime compiles it, and what is lost when that fails.
    site: &'static str,
    source: &'static str,
    name: PCSTR,
    entry: PCSTR,
    target: PCSTR,
}

/// A `main` compute shader for `cs_5_1`, as all of these are.
fn cs(what: &'static str, site: &'static str, source: &'static str, name: PCSTR) -> Helper {
    Helper {
        what,
        site,
        source,
        name,
        entry: s!("main"),
        target: s!("cs_5_1"),
    }
}

fn helpers() -> [Helper; 5] {
    [
        cs(
            "indirect argument clamp",
            "indirect::make_from; on failure every indirect draw and dispatch is dropped",
            crate::indirect::SOURCE,
            s!("clamp"),
        ),
        cs(
            "masked patch",
            "host::masked_patch_code (VICTORY_MASKED_PATCH, on by default); on failure the patch is not sent",
            crate::host::MASKED_PATCH,
            s!("masked patch"),
        ),
        cs(
            "card compare",
            "host::make_card_work; on failure no host copies are kept in the card's memory",
            crate::host::CARD_COMPARE,
            s!("card compare"),
        ),
        cs(
            "card compare, wide",
            "host::make_card_work and make_card_compare_wide (VICTORY_CARD_HASH_WIDE)",
            crate::host::CARD_COMPARE_WIDE,
            s!("card compare wide"),
        ),
        cs(
            "card fetch in flight",
            "host::make_card_inflight (VICTORY_CARD_FETCH_INFLIGHT=1); on failure the old fetch is used",
            crate::host::CARD_FETCH,
            s!("card fetch in flight"),
        ),
    ]
}

fn text(blob: &ID3DBlob) -> String {
    // SAFETY: the blob's own bytes, read while it is alive.
    let bytes = unsafe {
        std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize())
    };
    String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .trim()
        .to_owned()
}

/// The bytecode size, or the compiler's error and its messages.
fn compile(h: &Helper) -> Result<usize, String> {
    let (mut code, mut errors): (Option<ID3DBlob>, Option<ID3DBlob>) = (None, None);
    // SAFETY: the source and names outlive the call; out pointers ours.
    let result = unsafe {
        D3DCompile(
            h.source.as_ptr().cast(),
            h.source.len(),
            h.name,
            None,
            None,
            h.entry,
            h.target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        )
    };
    let messages = errors.as_ref().map(text).unwrap_or_default();
    match (result, code) {
        // SAFETY: a live blob.
        (Ok(()), Some(code)) => Ok(unsafe { code.GetBufferSize() }),
        (Ok(()), None) => Err(format!("no bytecode returned. {messages}")),
        (Err(e), _) => Err(format!("{e}. {messages}")),
    }
}

#[test]
fn runtime_helpers_compile_with_the_system_compiler() {
    let mut refused = Vec::new();
    for h in helpers() {
        match compile(&h) {
            Ok(size) => println!("hlsl-probe ok      {}: {size} bytes", h.what),
            Err(e) => {
                println!(
                    "hlsl-probe REFUSED {} ({}):\n    {}",
                    h.what,
                    h.site,
                    e.replace('\n', "\n    ")
                );
                refused.push(h.what);
            }
        }
    }
    assert!(
        refused.is_empty(),
        "the system HLSL compiler refused: {refused:?}"
    );
}
