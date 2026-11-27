//! The HLSL helpers the runtime compiles, each compiled the way its call site
//! does: same source name, entry point, target and flags, with the system's
//! `D3DCompile`. It needs no device. On Windows this guards the sources. Under
//! Wine or Proton it shows which helpers the builtin `d3dcompiler_47` refuses
//! (see docs/linux-proton.md). Run it with `--nocapture` to see each line.

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

fn helpers() -> [Helper; 4] {
    [
        Helper {
            what: "indirect argument clamp",
            site: "indirect::make; on failure every indirect draw and dispatch is dropped",
            source: crate::indirect::SOURCE,
            name: s!("clamp"),
            entry: s!("main"),
            target: s!("cs_5_1"),
        },
        Helper {
            what: "depth twin converter",
            site: "twin::compile; on failure no depth twin is made",
            source: crate::twin::SOURCE,
            name: s!("depth"),
            entry: s!("main"),
            target: s!("cs_5_1"),
        },
        Helper {
            what: "present compositor (vs)",
            site: "host::Compositor::new; on failure the compositor is not created",
            source: crate::host::COMPOSITE_SOURCE,
            name: s!("present_composite"),
            entry: s!("vs"),
            target: s!("vs_5_1"),
        },
        Helper {
            what: "present compositor (ps)",
            site: "host::Compositor::new; on failure the compositor is not created",
            source: crate::host::COMPOSITE_SOURCE,
            name: s!("present_composite"),
            entry: s!("ps"),
            target: s!("ps_5_1"),
        },
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
