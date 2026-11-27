//! The HLSL helpers the runtime compiles. Each is compiled twice: by the
//! system's `D3DCompile` alone, as the call sites did before (same source
//! name, entry point, target and flags), to show what that compiler
//! refuses; and by the path the runtime takes now
//! ([`crate::hlsl::compile_compute`]: the system's compiler, then DXC),
//! which must succeed. It needs no device. Under Wine or Proton the builtin
//! `d3dcompiler_47` refuses most of them (see docs/linux-proton.md); set
//! `VICTORY_DXC` to `dxcompiler.dll` or its folder. Run it with
//! `--nocapture` to see each line.
//!
//! Not listed: the opt-in `VICTORY_INPLACE_PROBE` shader, and the depth
//! plane's, which `host::compile` makes with the system's compiler.

struct Helper {
    what: &'static str,
    /// Where the runtime compiles it, and what is lost when that fails.
    site: &'static str,
    source: &'static str,
    /// The source name its call site passes.
    name: &'static str,
}

fn helpers() -> [Helper; 5] {
    [
        Helper {
            what: "indirect argument clamp",
            site: "indirect::make_from; on failure every indirect draw and dispatch is dropped",
            source: crate::indirect::SOURCE,
            name: "clamp",
        },
        Helper {
            what: "masked patch",
            site: "host::masked_patch_code (VICTORY_MASKED_PATCH, on by default); on failure the patch is not sent",
            source: crate::host::MASKED_PATCH,
            name: "masked patch",
        },
        Helper {
            what: "card compare",
            site: "host::make_card_work; on failure no host copies are kept in the card's memory",
            source: crate::host::CARD_COMPARE,
            name: "card compare",
        },
        Helper {
            what: "card compare, wide",
            site: "host::make_card_work and make_card_compare_wide (VICTORY_CARD_HASH_WIDE)",
            source: crate::host::CARD_COMPARE_WIDE,
            name: "card compare wide",
        },
        Helper {
            what: "card fetch in flight",
            site: "host::make_card_inflight (VICTORY_CARD_FETCH_INFLIGHT=1); on failure the old fetch is used",
            source: crate::host::CARD_FETCH,
            name: "card fetch in flight",
        },
    ]
}

#[test]
fn runtime_helpers_compile() {
    let mut failed = Vec::new();
    for h in helpers() {
        match crate::hlsl::system_compile(h.name, h.source) {
            Ok(code) => println!(
                "hlsl-probe system  ok      {}: {} bytes",
                h.what,
                code.len()
            ),
            Err((e, messages)) => println!(
                "hlsl-probe system  REFUSED {} ({}):\n    {e}. {}",
                h.what,
                h.site,
                messages.trim().replace('\n', "\n    ")
            ),
        }
        match crate::hlsl::compile_compute_by(h.name, h.source) {
            Ok((code, by)) => {
                assert_eq!(&code[..4], b"DXBC", "{}: not a shader container", h.what);
                println!(
                    "hlsl-probe runtime ok      {}: {} bytes by {by:?}",
                    h.what,
                    code.len()
                );
            }
            Err(e) => {
                println!("hlsl-probe runtime FAILED  {}: {e}", h.what);
                failed.push(h.what);
            }
        }
    }
    assert!(failed.is_empty(), "no compiler took: {failed:?}");
}
