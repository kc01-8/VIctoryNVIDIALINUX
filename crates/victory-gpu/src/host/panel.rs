//! The run's numbers on the picture, in every build (`VICTORY_PANEL=1`; off
//! by default; F3 or the settings menu shows and hides it while our window
//! has the focus; the time spent recording is counted in diagnostics builds
//! only): a panel in the upper left corner of what is shown, so that a
//! screenshot carries them, with what it costs itself on the submitting
//! thread. It shows the frame rate, a graph of the last frames' times and
//! one of the frame rate over the last minute, every processor's load, the
//! card's, its memory and the PC's, and a frame's work ([`super::hud`], in
//! the system's own fonts); where no font can be had, the same numbers in
//! white dot letters on a black box. Made again twice a second from the counters of
//! the frame-rate line, memory and textures; a frame itself only copies the
//! box out of a buffer in the card's memory. Copied onto the swap chain's
//! buffer, behind the title's picture: saved frames and the frame recorder
//! see the title's own.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_F3};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

use super::font::{colours, glyph};
use super::{Host, HostList, transition};

/// How often the numbers change.
const EVERY: Duration = Duration::from_millis(250);

/// Frames' times kept for the graph, and frame rates (one each [`EVERY`])
/// for the last minute.
const FRAMES_KEPT: usize = 240;
const RATES_KEPT: usize = 240;

/// A character's dots, and with the space beside and below it.
const GLYPH: (u32, u32) = (5, 7);
const CELL: (u32, u32) = (6, 9);
/// Characters in a line, lines, and the box's rim in dots.
const COLUMNS: u32 = 40;
const LINES: u32 = 10;
const RIM: u32 = 3;

/// The texts are put into one of these in turn: the card reads the others,
/// at most half a second old.
const UPLOADS: usize = 4;

fn asked() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| crate::switch::on("VICTORY_PANEL"))
}

/// Whether it is shown now.
pub(super) fn shown() -> &'static AtomicBool {
    static SHOWN: OnceLock<AtomicBool> = OnceLock::new();
    SHOWN.get_or_init(|| AtomicBool::new(asked()))
}

/// F3 was pressed since the last call, with one of our windows in front.
fn pressed() -> bool {
    static DOWN: AtomicBool = AtomicBool::new(false);
    // SAFETY: a plain query.
    let down = unsafe { GetAsyncKeyState(i32::from(VK_F3)) } < 0;
    if DOWN.swap(down, Ordering::Relaxed) || !down {
        return false;
    }
    let mut process = 0;
    // SAFETY: plain queries; room for the answer.
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), &mut process) };
    process == std::process::id()
}

/// A box's characters in a line, and lines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Shape {
    columns: u32,
    lines: u32,
}

/// The numbers panel's box.
const PANEL: Shape = Shape { columns: COLUMNS, lines: LINES };

/// The box's size in pixels, a dot being `scale` of them each way.
const fn size(shape: Shape, scale: u32) -> (u32, u32) {
    ((shape.columns * CELL.0 + 2 * RIM) * scale, (shape.lines * CELL.1 + 2 * RIM) * scale)
}

/// Pixels to a dot for a picture that high: a 720-line screenshot of it
/// shows one and a third.
fn scale(height: u32) -> u32 {
    (height / 540).max(1)
}

/// Whether a dot of the picture of `lines` is white.
fn lit(lines: &[String], shape: Shape, x: u32, y: u32) -> bool {
    let (Some(x), Some(y)) = (x.checked_sub(RIM), y.checked_sub(RIM)) else {
        return false;
    };
    if x % CELL.0 >= GLYPH.0 || y % CELL.1 >= GLYPH.1 || x / CELL.0 >= shape.columns {
        return false;
    }
    let Some(&c) = lines.get((y / CELL.1) as usize).and_then(|l| l.as_bytes().get((x / CELL.0) as usize)) else {
        return false;
    };
    (glyph(c.to_ascii_uppercase())[(y % CELL.1) as usize] >> (GLYPH.0 - 1 - x % CELL.0)) & 1 != 0
}

/// The box as rows of `pitch` bytes.
fn painted(lines: &[String], shape: Shape, scale: u32, colours: [&[u8]; 2], pitch: usize) -> Vec<u8> {
    let (width, height) = size(shape, scale);
    let bytes = colours[0].len();
    let mut out = vec![0; pitch * height as usize];
    for y in 0..height / scale {
        let top = pitch * (y * scale) as usize;
        for x in 0..width / scale {
            let colour = colours[usize::from(lit(lines, shape, x, y))];
            for i in 0..scale {
                let at = top + (x * scale + i) as usize * bytes;
                out[at..at + bytes].copy_from_slice(colour);
            }
        }
        for i in 1..scale as usize {
            out.copy_within(top..top + pitch, top + i * pitch);
        }
    }
    out
}

/// The counters read for it. They grow, and go back to 0 with the frame-rate
/// line (right behind a look of ours).
#[derive(Clone, Copy, Default)]
struct Counters([u64; COUNTERS]);

const COUNTERS: usize = 11;

impl Counters {
    fn now() -> Counters {
        use super::{CARD_CLOCK_US, COPY_BACK, PREPARING};
        #[cfg(feature = "diagnostics")]
        let recording = crate::copystat::peek(crate::copystat::C::RecordNs);
        #[cfg(not(feature = "diagnostics"))]
        let recording = 0;
        let get = |a: &std::sync::atomic::AtomicU64| a.load(Ordering::Relaxed);
        Counters([
            super::worked(),
            get(&CARD_CLOCK_US[0]),
            get(&CARD_CLOCK_US[1]),
            get(&CARD_CLOCK_US[2]),
            get(&PREPARING[3]),
            get(&COPY_BACK[0]),
            get(&COPY_BACK[1]),
            get(&crate::shadow::CARD_STATS[0]),
            crate::draw::dropped_now(),
            recording,
            crate::draw::drawn(),
        ])
    }

    /// What came since `before`, per frame.
    fn since(self, before: Counters, frames: f64) -> [f64; COUNTERS] {
        let mut out = [0.0; COUNTERS];
        for (i, out) in out.iter_mut().enumerate() {
            let (was, is) = (before.0[i], self.0[i]);
            *out = if is >= was { is - was } else { is } as f64 / frames;
        }
        out
    }
}

/// The process's memory in bytes: in physical memory, and committed
/// (private bytes).
fn process_memory() -> (u64, u64) {
    use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    let mut c = PROCESS_MEMORY_COUNTERS {
        cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    // SAFETY: our own process and a sized structure.
    unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) };
    (c.WorkingSetSize as u64, c.PagefileUsage as u64)
}

/// The frame generation line: `per` frames shown for each one rendered.
fn generated(fps: f64, per: u32) -> String {
    match per {
        0 | 1 => "FRAME GEN OFF".to_owned(),
        per => format!("FRAME GEN {per}X  SHOWN {:.1} FPS", fps * f64::from(per)),
    }
}

/// The text. `c`: [`Counters`] per frame, `ms` a frame's time, `worst` the
/// longest frame's, `cost` what this takes of a frame, in microseconds.
fn text(c: &[f64; COUNTERS], ms: f64, worst: f64, cost: f64, host: &Host) -> Vec<String> {
    let gb = |bytes: u64| bytes as f64 / (1u64 << 30) as f64;
    let (working, private) = process_memory();
    let [budget, used, _, shared] = host.video_memory().unwrap_or_default();
    let idle = (c[1] / 1e3).min(ms);
    let fps = 1e3 / ms.max(0.001);
    vec![
        format!("FPS {fps:.1}  FRAME {ms:.1} MS  LONGEST {worst:.0}"),
        generated(fps, crate::streamline::shown_per_frame()),
        format!("DRAWS {:.0}  DROPPED {:.0}", c[10], c[8]),
        format!("SUBMIT BUSY {:.1}  WAITS FOR GPU {:.1}", c[0] / 1e3, c[3] / 1e3),
        format!("GPU BUSY {:.1}  IDLE BEHIND US {:.1}", ms - idle, c[2] / 1e3),
        if cfg!(feature = "diagnostics") {
            format!("RECORDING {:.0} MS  COPY BACK {:.1}", c[9] / 1e6, c[5] / 1e3)
        } else {
            format!("COPY BACK {:.1} MS", c[5] / 1e3)
        },
        format!("MB UP {:.0}  TEXTURES {:.0}  BACK {:.0}", c[7] / 1e6, c[4] / 1e6, c[6] / 1e6),
        format!("RAM {:.1} GB  IN MEMORY {:.1}", gb(private), gb(working)),
        format!("VRAM {:.1}/{:.1} GB  SHARED {:.1}", gb(used), gb(budget), gb(shared)),
        format!("THIS PANEL {:.0} US A FRAME", cost),
    ]
}

/// The buffers of a box that size and format.
struct Buffers {
    size: (u32, u32),
    format: DXGI_FORMAT,
    /// What they were made for (the dot scale, or the panel's scale in
    /// thousandths).
    scale: u32,
    pitch: u32,
    uploads: Vec<(ID3D12Resource, *mut u8)>,
    card: ID3D12Resource,
    /// The upload written last.
    turn: usize,
    /// It is not in the card's buffer yet.
    fresh: bool,
}

// SAFETY: free-threaded resources, and memory only written under the presenter's lock.
unsafe impl Send for Buffers {}

impl Buffers {
    fn new(host: &Host, (width, height): (u32, u32), format: DXGI_FORMAT, scale: u32) -> Option<Buffers> {
        let pitch = (width * colours(format)?[0].len() as u32).next_multiple_of(256);
        let bytes = u64::from(pitch) * u64::from(height);
        let uploads = (0..UPLOADS).map(|_| host.upload_buffer(bytes)).collect::<Option<Vec<_>>>()?;
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: bytes,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };
        let mut card: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a buffer.
        unsafe {
            host.device
                .CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_RESOURCE_STATE_COMMON, None, &mut card)
                .ok()?;
        }
        Some(Buffers { size: (width, height), format, scale, pitch, uploads, card: card?, turn: 0, fresh: false })
    }

    /// The dot letters of `lines` in a box of `shape`.
    fn write(&mut self, lines: &[String], shape: Shape) {
        let Some(colours) = colours(self.format) else { return };
        let picture = painted(lines, shape, self.scale, colours, self.pitch as usize);
        self.write_picture(&picture);
    }

    /// A picture of rows of the buffers' pitch.
    fn write_picture(&mut self, picture: &[u8]) {
        let bytes = (self.pitch * self.size.1) as usize;
        if picture.len() != bytes {
            return;
        }
        self.turn = (self.turn + 1) % UPLOADS;
        // SAFETY: a mapped upload buffer of the picture's size, which the card read a second or more ago.
        unsafe { std::ptr::copy_nonoverlapping(picture.as_ptr(), self.uploads[self.turn].1, bytes) };
        self.fresh = true;
    }

    /// Records the copy of the box onto `back` (of `width` x `height`, in the
    /// state to be copied into), its upper left corner at `at`.
    fn copy(&mut self, list: &ID3D12GraphicsCommandList, back: &ID3D12Resource, at: (u32, u32), (width, height): (u32, u32)) {
        let (w, h) = self.size;
        if at.0 >= width || at.1 >= height {
            return;
        }
        // SAFETY: an open list and live resources; the box lies within both.
        unsafe {
            if std::mem::take(&mut self.fresh) {
                list.CopyResource(&self.card, &self.uploads[self.turn].0);
                list.ResourceBarrier(&[transition(&self.card, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COPY_SOURCE)]);
            }
            let from = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(&self.card),
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                        Offset: 0,
                        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                            Format: self.format,
                            Width: w,
                            Height: h,
                            Depth: 1,
                            RowPitch: self.pitch,
                        },
                    },
                },
            };
            let to = D3D12_TEXTURE_COPY_LOCATION {
                pResource: std::mem::transmute_copy(back),
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
            };
            let part = D3D12_BOX { left: 0, top: 0, front: 0, right: w.min(width - at.0), bottom: h.min(height - at.1), back: 1 };
            list.CopyTextureRegion(&to, at.0, at.1, 0, &from, Some(&part));
        }
    }
}

impl Buffers {
    /// Records the pass that blends the picture (straight RGBA) over `back`
    /// at its upper left corner: `back` in the state to be copied into, as
    /// [`Buffers::copy`] has it, before and after.
    fn blend(
        &mut self,
        host: &Host,
        list: &ID3D12GraphicsCommandList,
        back: &ID3D12Resource,
        (width, height): (u32, u32),
        format: DXGI_FORMAT,
        (root, pipeline): &(ID3D12RootSignature, ID3D12PipelineState),
    ) {
        let (w, h) = (self.size.0.min(width), self.size.1.min(height));
        // scRGB takes linear light; the other formats the bytes as they are.
        let linear = u32::from(format == DXGI_FORMAT_R16G16B16A16_FLOAT);
        let constants = [0u32, 0, w, h, self.pitch, linear];
        // SAFETY: an open list and live resources; the view is recorded at once.
        unsafe {
            if std::mem::take(&mut self.fresh) {
                list.CopyResource(&self.card, &self.uploads[self.turn].0);
                list.ResourceBarrier(&[transition(&self.card, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE)]);
            }
            list.ResourceBarrier(&[transition(back, D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_RENDER_TARGET)]);
            let rtv = host.rtv(Some(back), None);
            list.OMSetRenderTargets(1, Some(&rtv), false, None);
            list.SetGraphicsRootSignature(root);
            list.SetPipelineState(pipeline);
            list.SetGraphicsRoot32BitConstants(0, constants.len() as u32, constants.as_ptr().cast(), 0);
            list.SetGraphicsRootShaderResourceView(1, self.card.GetGPUVirtualAddress());
            list.RSSetViewports(&[D3D12_VIEWPORT { TopLeftX: 0.0, TopLeftY: 0.0, Width: width as f32, Height: height as f32, MinDepth: 0.0, MaxDepth: 1.0 }]);
            list.RSSetScissorRects(&[windows::Win32::Foundation::RECT { left: 0, top: 0, right: w as i32, bottom: h as i32 }]);
            list.IASetPrimitiveTopology(windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            list.DrawInstanced(3, 1, 0, 0);
            list.ResourceBarrier(&[transition(back, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATE_COPY_DEST)]);
        }
    }
}

/// The pass that blends the panel over the picture: one triangle over the
/// whole of it, cut to the panel by the scissor; the pixel shader reads the
/// panel's straight RGBA through a root view of its buffer (no descriptor
/// heap) and gives it premultiplied, blended as such.
const BLEND: &str = r#"
#define RS "RootConstants(num32BitConstants=6, b0), SRV(t0, visibility = SHADER_VISIBILITY_PIXEL)"
cbuffer Place : register(b0) { uint2 origin; uint2 size; uint pitch; uint linear_light; };
ByteAddressBuffer picture : register(t0);
[RootSignature(RS)]
float4 corner(uint i : SV_VertexID) : SV_Position {
    float2 uv = float2((i << 1) & 2, i & 2);
    return float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
}
[RootSignature(RS)]
float4 panel(float4 p : SV_Position) : SV_Target {
    uint2 at = uint2(p.xy) - origin;
    if (at.x >= size.x || at.y >= size.y) discard;
    uint word = picture.Load(at.y * pitch + at.x * 4);
    float4 c = float4(word & 0xff, (word >> 8) & 0xff, (word >> 16) & 0xff, word >> 24) / 255.0;
    if (linear_light != 0) {
        float3 low = c.rgb / 12.92;
        float3 high = pow((c.rgb + 0.055) / 1.055, 2.4);
        c.rgb = lerp(low, high, step(0.04045, c.rgb));
    }
    return float4(c.rgb * c.a, c.a);
}
"#;

/// The blending pass's root signature and its pipelines by format.
struct Blender {
    root: ID3D12RootSignature,
    vs: Vec<u8>,
    ps: Vec<u8>,
    made: std::sync::Mutex<Vec<(i32, Option<(ID3D12RootSignature, ID3D12PipelineState)>)>>,
}

// SAFETY: D3D12 objects are free-threaded.
unsafe impl Send for Blender {}
// SAFETY: as above.
unsafe impl Sync for Blender {}

/// Whether the panel is blended over the picture with its translucent
/// ground (`VICTORY_PANEL_BLEND=0`: copied over it, opaque). (Runs 47 and
/// 48 stopped early with it, but on unhandled guard faults that came of
/// `VICTORY_FAULT_UNLOCKED`, not of this.)
fn blend_asked() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_PANEL_BLEND").is_none_or(|v| v != "0"))
}

/// The blending pass, compiled once; `None` if it cannot be.
fn blender() -> Option<&'static Blender> {
    static BLENDER: OnceLock<Option<Blender>> = OnceLock::new();
    BLENDER
        .get_or_init(|| {
            use windows::core::s;
            let host = super::host()?;
            let made = (|| -> windows::core::Result<Blender> {
                let ps = super::compile(BLEND, s!("panel"), s!("ps_5_1"))?;
                // Both stages of one kind (as the depth planes' pass).
                let vs = if crate::hlsl::is_dxil(&ps) {
                    crate::hlsl::dxc_compile_for("the panel's corners", BLEND, "corner", "vs_6_0")?
                } else {
                    super::compile(BLEND, s!("corner"), s!("vs_5_1"))?
                };
                // SAFETY: the root signature is inside the pixel shader's code.
                let root: ID3D12RootSignature = unsafe { host.device.CreateRootSignature(0, &ps)? };
                Ok(Blender { root, vs, ps, made: std::sync::Mutex::default() })
            })();
            match made {
                Ok(b) => Some(b),
                Err(e) => {
                    tracing::warn!(error = %e, "the numbers panel is copied over the picture: its blending pass could not be compiled");
                    None
                }
            }
        })
        .as_ref()
}

impl Blender {
    /// The pipeline for a picture of `format`, made once.
    fn pipeline(&self, host: &Host, format: DXGI_FORMAT) -> Option<(ID3D12RootSignature, ID3D12PipelineState)> {
        let mut made = self.made.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, p)) = made.iter().find(|(f, _)| *f == format.0) {
            return p.clone();
        }
        let code = |bytes: &[u8]| D3D12_SHADER_BYTECODE { pShaderBytecode: bytes.as_ptr().cast(), BytecodeLength: bytes.len() };
        let target = D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true.into(),
            SrcBlend: D3D12_BLEND_ONE,
            DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D12_BLEND_OP_ADD,
            SrcBlendAlpha: D3D12_BLEND_ONE,
            DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D12_BLEND_OP_ADD,
            LogicOp: D3D12_LOGIC_OP_NOOP,
            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
            ..Default::default()
        };
        let mut formats = [DXGI_FORMAT_UNKNOWN; 8];
        formats[0] = format;
        // SAFETY: the description points at live data and never releases its pointer.
        let pipeline = unsafe {
            let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: std::mem::transmute_copy(&self.root),
                VS: code(&self.vs),
                PS: code(&self.ps),
                BlendState: D3D12_BLEND_DESC { RenderTarget: [target; 8], ..Default::default() },
                SampleMask: u32::MAX,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                RTVFormats: formats,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                ..Default::default()
            };
            host.device.CreateGraphicsPipelineState::<ID3D12PipelineState>(&desc)
        };
        let p = match pipeline {
            Ok(pso) => Some((self.root.clone(), pso)),
            Err(e) => {
                tracing::warn!(error = %e, format = format.0, "the numbers panel is copied over the picture: no blending pipeline for this format");
                None
            }
        };
        made.push((format.0, p.clone()));
        p
    }
}

/// The panel's numbers the system is asked for, on the painter's thread:
/// each processor's time, the process's, its memory and the PC's, the card's
/// memory. Under Wine these go through `/proc` and the wineserver, and on
/// the presenting thread they were most of a frame's chain (with the F1
/// menu and the frame meter, 762 ms in 6 frames, run 67's timeline).
struct SystemNumbers {
    processors: Vec<(u64, u64)>,
    process: (u64, Instant),
}

impl SystemNumbers {
    fn new() -> SystemNumbers {
        SystemNumbers { processors: processor_times(), process: (process_time(), Instant::now()) }
    }

    /// Puts them in `s`.
    fn fill(&mut self, s: &mut super::hud::Snapshot) {
        let now = Instant::now();
        let processors = processor_times();
        let (mut busy, mut all) = (0u64, 0u64);
        s.cores.clear();
        for (i, &(b, t)) in processors.iter().enumerate() {
            let (b0, t0) = self.processors.get(i).copied().unwrap_or((b, t));
            let (db, dt) = (b.saturating_sub(b0), t.saturating_sub(t0));
            s.cores.push(if dt > 0 { (db as f64 / dt as f64) as f32 } else { 0.0 });
            (busy, all) = (busy + db, all + dt);
        }
        self.processors = processors;
        s.system = if all > 0 { busy as f64 / all as f64 } else { 0.0 };
        let process = process_time();
        let wall = (now - self.process.1).as_secs_f64();
        s.ours = if wall > 0.0 { process.saturating_sub(self.process.0) as f64 / 1e7 / wall } else { 0.0 };
        self.process = (process, now);
        let (working, private) = process_memory();
        s.ram = (linux_resident().unwrap_or(private), working, physical_memory());
        if let Some(host) = super::host() {
            let [budget, used, _, _] = host.video_memory().unwrap_or_default();
            s.vram = (used, budget);
        }
    }
}

/// What the painter is asked for: the numbers, the scale, the pixels'
/// format (RGBA for the blending pass) and pitch, and what tells the picture
/// for those apart.
struct Job {
    snapshot: super::hud::Snapshot,
    scale: f32,
    format: DXGI_FORMAT,
    pitch: usize,
    key: (u32, i32, u32),
}

/// The thread that paints the panel: the presenting thread hands it the
/// numbers four times a second and takes the last picture it finished.
struct Painter {
    job: std::sync::Mutex<Option<Job>>,
    asked: std::sync::Condvar,
    done: std::sync::Mutex<Option<((u32, i32, u32), Vec<u8>)>>,
    /// No fonts could be had: the dot letters.
    no_fonts: AtomicBool,
}

impl Painter {
    fn start() -> std::sync::Arc<Painter> {
        let painter = std::sync::Arc::new(Painter {
            job: std::sync::Mutex::new(None),
            asked: std::sync::Condvar::new(),
            done: std::sync::Mutex::new(None),
            no_fonts: AtomicBool::new(false),
        });
        let p = painter.clone();
        let started = std::thread::Builder::new().name("victory-panel".into()).spawn(move || {
            let mut fonts: Option<super::hud::Type> = None;
            let mut numbers = SystemNumbers::new();
            loop {
                let job = {
                    let mut job = p.job.lock().unwrap_or_else(|e| e.into_inner());
                    while job.is_none() {
                        job = p.asked.wait(job).unwrap_or_else(|e| e.into_inner());
                    }
                    job.take().expect("asked for")
                };
                if fonts.as_ref().is_none_or(|t| t.scale() != job.scale) {
                    let (w, _) = super::hud::size(job.scale);
                    fonts = super::hud::Type::new((w, (72.0 * job.scale).ceil() as u32), job.scale);
                    if fonts.is_none() {
                        tracing::info!("no fonts for the numbers panel: its dot letters instead");
                        p.no_fonts.store(true, Ordering::Relaxed);
                        return;
                    }
                }
                let Some(t) = fonts.as_mut() else { return };
                let mut snapshot = job.snapshot;
                numbers.fill(&mut snapshot);
                let picture = super::hud::paint(t, &snapshot);
                let bytes = if job.format == DXGI_FORMAT_R8G8B8A8_UNORM && blend_asked() {
                    Some(picture.rgba(job.pitch))
                } else {
                    picture.encode(job.format, job.pitch)
                };
                save_now_and_then(&picture);
                if let Some(bytes) = bytes {
                    *p.done.lock().unwrap_or_else(|e| e.into_inner()) = Some((job.key, bytes));
                }
            }
        });
        if started.is_err() {
            painter.no_fonts.store(true, Ordering::Relaxed);
        }
        painter
    }

    /// Asks for a picture of `job` (one asked for before and not begun is
    /// dropped: only the newest counts).
    fn ask(&self, job: Job) {
        *self.job.lock().unwrap_or_else(|e| e.into_inner()) = Some(job);
        self.asked.notify_one();
    }

    /// The last picture finished, if one for `key` came since the last take.
    fn take(&self, key: (u32, i32, u32)) -> Option<Vec<u8>> {
        let mut done = self.done.lock().unwrap_or_else(|e| e.into_inner());
        match done.take() {
            Some((k, bytes)) if k == key => Some(bytes),
            _ => None,
        }
    }
}

/// The panel of a presenter.
pub(super) struct Panel {
    since: Instant,
    frames: u32,
    last: Instant,
    worst: f64,
    /// What this took since the last text.
    cost: Duration,
    before: Counters,
    lines: Vec<String>,
    changed: bool,
    buffers: Option<Buffers>,
    failed: bool,
    /// The numbers as the panel shows them, and what they need kept.
    snapshot: super::hud::Snapshot,
    times: std::collections::VecDeque<f32>,
    rates: std::collections::VecDeque<f32>,

    /// The fonts, once made; whether they could not be.
    /// The thread that paints the panel, once started.
    painter: Option<std::sync::Arc<Painter>>,
}

impl Panel {
    pub(super) fn new() -> Panel {
        let now = Instant::now();
        Panel {
            since: now,
            frames: 0,
            last: now,
            worst: 0.0,
            cost: Duration::ZERO,
            before: Counters::default(),
            lines: Vec::new(),
            changed: false,
            buffers: None,
            failed: false,
            snapshot: super::hud::Snapshot::default(),
            times: std::collections::VecDeque::with_capacity(FRAMES_KEPT),
            rates: std::collections::VecDeque::with_capacity(RATES_KEPT),

            painter: None,
        }
    }

    /// A frame was presented; `line`: the counters go back to 0 right behind
    /// this call. (Not shown it asks for the key's state and no more.)
    pub(super) fn frame(&mut self, line: bool) {
        if pressed() {
            shown().fetch_xor(true, Ordering::Relaxed);
        }
        if !shown().load(Ordering::Relaxed) {
            self.frames = 0;
            return;
        }
        let now = Instant::now();
        if self.frames == 0 && now - self.last > 2 * EVERY {
            // (Shown from here on.)
            (self.since, self.last, self.worst, self.before) = (now, now, 0.0, Counters::now());
            self.frames = 0;
        }
        self.frames += 1;
        let took = (now - self.last).as_secs_f64() * 1e3;
        self.worst = self.worst.max(took);
        self.last = now;
        if self.frames > 1 || took < 2e3 {
            if self.frame_times_full() {
                self.frame_times_pop();
            }
            self.frame_times_push(took as f32);
        }
        let elapsed = now - self.since;
        if elapsed < EVERY && !(line && elapsed * 2 >= EVERY) {
            return;
        }
        let Some(host) = super::host() else { return };
        let frames = f64::from(self.frames);
        let counters = Counters::now();
        let c = counters.since(self.before, frames);
        self.before = counters;
        let cost = self.cost.as_secs_f64() * 1e6 / frames;
        let ms = elapsed.as_secs_f64() * 1e3 / frames;
        self.lines = text(&c, ms, self.worst, cost, host);
        self.snapshot = self.snapshot(&c, ms, cost, host, now);
        self.changed = true;
        (self.since, self.frames, self.worst) = (now, 0, 0.0);
        self.cost = now.elapsed();
    }

    /// Records the copy of the box onto `back`, a buffer of the swap chain of
    /// that size and format in the state to be copied into.
    pub(super) fn draw(
        &mut self,
        host: &Host,
        list: &HostList,
        back: &ID3D12Resource,
        (width, height): (u32, u32),
        format: DXGI_FORMAT,
    ) {
        if !shown().load(Ordering::Relaxed) || self.lines.is_empty() || self.failed {
            return;
        }
        let began = Instant::now();
        // Not the title's: the frame recorder leaves it out.
        #[cfg(feature = "diagnostics")]
        let _ours = crate::replay::record::unrecorded();
        // The panel in the system's fonts, painted on a thread of its own
        // (it took 7.3 ms a frame on this one, run 57); else the dot letters.
        let k = super::hud::scale(height);
        let painter = self.painter.get_or_insert_with(Painter::start).clone();
        let fancy = !painter.no_fonts.load(Ordering::Relaxed) && colours(format).is_some();
        // Blended onto the picture where the pass can be made (its buffers
        // hold RGBA), else copied over it.
        let blended = fancy && blend_asked() && blender().is_some_and(|b| b.pipeline(host, format).is_some());
        let held = if blended { DXGI_FORMAT_R8G8B8A8_UNORM } else { format };
        let (size, scale) = if fancy {
            (super::hud::size(k), (k * 1000.0).round() as u32 * 2 + u32::from(blended))
        } else {
            let scale = scale(height);
            (size(PANEL, scale), scale)
        };
        if self.buffers.as_ref().is_none_or(|b| (b.format, b.scale, b.size) != (held, scale, size)) {
            // (What the card may still read stays: a picture's size changes once or twice in a run.)
            std::mem::forget(self.buffers.take());
            self.buffers = Buffers::new(host, size, held, scale);
            self.failed = self.buffers.is_none();
            self.changed = true;
        }
        let Some(buffers) = &mut self.buffers else { return };
        let key = (k.to_bits(), held.0, buffers.pitch);
        if std::mem::take(&mut self.changed) {
            if fancy {
                painter.ask(Job { snapshot: self.snapshot.clone(), scale: k, format: held, pitch: buffers.pitch as usize, key });
            } else {
                buffers.write(&self.lines, PANEL);
            }
        }
        if fancy && let Some(picture) = painter.take(key) {
            buffers.write_picture(&picture);
        }
        if blended && let Some(pipeline) = blender().and_then(|b| b.pipeline(host, format)) {
            buffers.blend(host, &list.list, back, (width, height), format, &pipeline);
            self.cost += began.elapsed();
            return;
        }
        buffers.copy(&list.list, back, (0, 0), (width, height));
        self.cost += began.elapsed();
    }
}

impl Panel {
    fn frame_times_full(&self) -> bool {
        self.times.len() >= FRAMES_KEPT
    }

    fn frame_times_pop(&mut self) {
        self.times.pop_front();
    }

    fn frame_times_push(&mut self, ms: f32) {
        self.times.push_back(ms);
    }

    /// The numbers for the panel's picture: `c` [`Counters`] per frame, `ms`
    /// a frame's time, `cost` the panel's own per frame.
    fn snapshot(&mut self, c: &[f64; COUNTERS], ms: f64, cost: f64, host: &Host, now: Instant) -> super::hud::Snapshot {
        let fps = 1e3 / ms.max(0.001);
        if self.rates.len() >= RATES_KEPT {
            self.rates.pop_front();
        }
        self.rates.push_back(fps as f32);
        let _ = (host, now);
        let mut sorted: Vec<f32> = self.times.iter().copied().collect();
        sorted.sort_by(f32::total_cmp);
        let slow = sorted.get(sorted.len().saturating_sub(1).min(sorted.len() * 99 / 100)).map_or(0.0, |&v| f64::from(v));
        let idle = (c[1] / 1e3).min(ms);
        super::hud::Snapshot {
            fps,
            ms,
            worst: self.worst,
            slow,
            frames: self.times.iter().copied().collect(),
            rates: self.rates.iter().copied().collect(),
            shown_per: crate::streamline::shown_per_frame().max(1),
            // (The system's numbers: the painter's thread asks for them,
            // [`SystemNumbers`].)
            cores: Vec::new(),
            system: 0.0,
            ours: 0.0,
            gpu: if ms > 0.0 { ((ms - idle) / ms).clamp(0.0, 1.0) } else { 0.0 },
            vram: (0, 0),
            ram: (0, 0, 0),
            draws: c[10],
            dropped: c[8],
            submit: c[0] / 1e3,
            waits: c[3] / 1e3,
            up: c[7] / 1e6,
            textures: c[4] / 1e6,
            back: c[5] / 1e3,
            making: crate::pipeline::making(),
            cost,
            tape: crate::feed::recent(12),
            counts: crate::feed::counts(),
        }
    }
}

/// With `VICTORY_HUD_PICTURE=<folder>`, the panel's own picture (nothing
/// of the title's) as `hud-live.ppm` there, every five seconds: to look at
/// it without a screenshot.
fn save_now_and_then(picture: &super::hud::Canvas) {
    static FOLDER: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    static LAST: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
    let Some(folder) = FOLDER.get_or_init(|| std::env::var_os("VICTORY_HUD_PICTURE").map(Into::into)) else { return };
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if last.is_some_and(|t| t.elapsed() < Duration::from_secs(5)) {
        return;
    }
    *last = Some(Instant::now());
    let mut ppm = format!("P6 {} {} 255\n", picture.width, picture.height).into_bytes();
    for y in 0..picture.height {
        for x in 0..picture.width {
            let super::hud::Rgb(r, g, b) = picture.at(x, y);
            ppm.extend([r, g, b]);
        }
    }
    let _ = std::fs::write(folder.join("hud-live.ppm"), ppm);
}

/// Each processor's busy and whole time since the start (100 ns), from
/// Windows (under Wine, from Linux's `/proc/stat`); empty if it says nothing.
fn processor_times() -> Vec<(u64, u64)> {
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    /// `SYSTEM_PROCESSOR_PERFORMANCE_INFORMATION`.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Times {
        idle: i64,
        kernel: i64,
        user: i64,
        dpc: i64,
        interrupt: i64,
        interrupts: u32,
    }
    type Query = unsafe extern "system" fn(u32, *mut core::ffi::c_void, u32, *mut u32) -> i32;
    static QUERY: OnceLock<Option<Query>> = OnceLock::new();
    // SAFETY: ntdll's own export, of this signature.
    let query = *QUERY.get_or_init(|| unsafe {
        let ntdll = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        if ntdll.is_null() {
            return None;
        }
        GetProcAddress(ntdll, c"NtQuerySystemInformation".as_ptr().cast()).map(|f| std::mem::transmute::<_, Query>(f))
    });
    let Some(query) = query else { return Vec::new() };
    let mut times = vec![Times::default(); 256];
    let mut got = 0u32;
    // SAFETY: room for 256 processors; class 8 is SystemProcessorPerformanceInformation.
    let status = unsafe { query(8, times.as_mut_ptr().cast(), (times.len() * size_of::<Times>()) as u32, &mut got) };
    if status < 0 {
        return Vec::new();
    }
    let n = (got as usize / size_of::<Times>()).min(times.len());
    times[..n]
        .iter()
        .map(|t| {
            // (Kernel time holds idle time.)
            let whole = (t.kernel.max(0) + t.user.max(0)) as u64;
            (whole.saturating_sub(t.idle.max(0) as u64), whole)
        })
        .collect()
}

/// The process's CPU time since its start (100 ns), all threads.
fn process_time() -> u64 {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut made, mut ended, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: our own process; room for the answers.
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut made, &mut ended, &mut kernel, &mut user) };
    let t = |f: FILETIME| u64::from(f.dwHighDateTime) << 32 | u64::from(f.dwLowDateTime);
    t(kernel) + t(user)
}

/// Under Wine, the memory Linux holds for this process that cannot be
/// given back without swap: its anonymous and shared memory (the title's
/// memory is in shared sections, which Windows' private bytes leave out),
/// each page once. `None` on Windows. (NVIDIA's pins of buffers are not in
/// it.)
///
/// Proportional sizes (`Pss_Anon`, `Pss_Shmem` of `smaps_rollup`) count a
/// page mapped at two addresses once, as the title's pages are (their views
/// and ours); `RssShmem` counts it twice (12.7 GB of shared memory against
/// the cgroup's 6.7 GB). Walking every mapping takes time (4.5 % of the
/// CPU's samples read every two seconds, run 56: the title has a great many
/// mappings), so a thread of its own reads it every fifteen seconds.
///
/// Not `/proc/self`: under Wine that is the wineserver's (a file opened
/// through drive Z: is looked up by it), so the panel showed its 16 MB.
/// Our own process is the one in `/proc` whose command line starts with our
/// own path (Wine names it `main`, after its main thread).
fn linux_resident() -> Option<u64> {
    use std::sync::atomic::AtomicU64;
    static HELD: AtomicU64 = AtomicU64::new(0);
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let Some(pid) = own_linux_process() else {
            tracing::info!("the numbers panel's RAM: Windows' private bytes (no Linux process of ours found)");
            return;
        };
        let rollup = format!(r"Z:\proc\{pid}\smaps_rollup");
        let read = move || -> Result<u64, String> {
            let text = std::fs::read_to_string(&rollup).map_err(|e| e.to_string())?;
            let field = |name: &str| -> Option<u64> {
                let line = text.lines().find(|l| l.starts_with(name))?;
                line[name.len()..].trim().trim_end_matches("kB").trim().parse::<u64>().ok().map(|kb| kb * 1024)
            };
            match (field("Pss_Anon:"), field("Pss_Shmem:")) {
                (Some(anon), shmem) => Ok(anon + shmem.unwrap_or(0)),
                _ => Err(format!("no Pss_Anon in {} bytes", text.len())),
            }
        };
        match read() {
            Ok(bytes) => {
                tracing::info!(pid, mb = bytes >> 20, "the numbers panel's RAM: the process's anonymous and shared memory, from Linux");
                HELD.store(bytes, Ordering::Relaxed);
                let _ = std::thread::Builder::new().name("victory-panel-ram".into()).spawn(move || {
                    loop {
                        std::thread::sleep(Duration::from_secs(15));
                        match read() {
                            Ok(bytes) => HELD.store(bytes, Ordering::Relaxed),
                            Err(_) => break,
                        }
                    }
                });
            }
            Err(e) => tracing::info!(error = %e, "the numbers panel's RAM: Windows' private bytes (no Linux books)"),
        }
    });
    let held = HELD.load(Ordering::Relaxed);
    (held > 0).then_some(held)
}

/// Under Wine, the Linux process this is: of the processes in `/proc` whose
/// command line starts with our own path (as Windows names it, `Z:\...`),
/// the one with the most threads; `None` without a `/proc` (Windows).
fn own_linux_process() -> Option<u32> {
    let own = std::env::current_exe().ok()?.to_string_lossy().into_owned();
    let mut best: Option<(u32, u32)> = None;
    for entry in std::fs::read_dir(r"Z:\proc").ok()?.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else { continue };
        let Ok(command) = std::fs::read(format!(r"Z:\proc\{pid}\cmdline")) else { continue };
        let first = command.split(|&b| b == 0).next().unwrap_or_default();
        if !String::from_utf8_lossy(first).eq_ignore_ascii_case(&own) {
            continue;
        }
        let Ok(status) = std::fs::read_to_string(format!(r"Z:\proc\{pid}\status")) else { continue };
        let field = |name: &str| status.lines().find(|l| l.starts_with(name)).map(|l| l[name.len()..].trim().to_owned());
        let threads = field("Threads:").and_then(|t| t.parse::<u32>().ok()).unwrap_or(0);
        if best.is_none_or(|(_, most)| threads > most) {
            best = Some((pid, threads));
        }
    }
    best.map(|(pid, _)| pid)
}

/// The PC's memory in bytes.
fn physical_memory() -> u64 {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: a sized structure.
    unsafe {
        let mut m: MEMORYSTATUSEX = std::mem::zeroed();
        m.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        GlobalMemoryStatusEx(&mut m);
        m.ullTotalPhys
    }
}

/// How long the start's notice stays.
const NOTICE_FOR: Duration = Duration::from_secs(30);
/// The longest notice line, in characters.
const NOTICE_COLUMNS: u32 = 52;

/// What this card does for frame generation, said at the top of the picture
/// for the first [`NOTICE_FOR`] of a run (`crate::streamline::notice`); in
/// every build, nothing without a notice.
pub(super) struct Notice {
    lines: Vec<String>,
    shape: Shape,
    since: Option<Instant>,
    buffers: Option<Buffers>,
    written: bool,
    done: bool,
}

impl Notice {
    pub(super) fn new() -> Notice {
        // The first start of a new victory.exe says first that it restarts by itself (menu.rs).
        let warmup = super::menu::warmup_seconds().map(super::menu::warmup_notice).unwrap_or_default();
        let lines: Vec<String> =
            warmup.into_iter().chain(crate::streamline::notice()).map(|l| l.to_ascii_uppercase()).collect();
        let columns = lines.iter().map(|l| l.len() as u32).max().unwrap_or(0).min(NOTICE_COLUMNS);
        let shape = Shape { columns, lines: lines.len() as u32 };
        Notice { done: lines.is_empty(), lines, shape, since: None, buffers: None, written: false }
    }

    /// Records the copy of the notice onto `back` (as [`Panel::draw`]), at
    /// the top in the middle, while it is due.
    pub(super) fn draw(&mut self, host: &Host, list: &HostList, back: &ID3D12Resource, (width, height): (u32, u32), format: DXGI_FORMAT) {
        if self.done {
            return;
        }
        let since = *self.since.get_or_insert_with(Instant::now);
        if since.elapsed() > NOTICE_FOR {
            // (Its buffers stay: the card may still read them.)
            std::mem::forget(self.buffers.take());
            self.done = true;
            return;
        }
        #[cfg(feature = "diagnostics")]
        let _ours = crate::replay::record::unrecorded();
        let scale = scale(height);
        if self.buffers.as_ref().is_none_or(|b| (b.format, b.scale) != (format, scale)) {
            std::mem::forget(self.buffers.take());
            self.buffers = Buffers::new(host, size(self.shape, scale), format, scale);
            self.written = false;
            if self.buffers.is_none() {
                self.done = true;
                return;
            }
        }
        let Some(buffers) = &mut self.buffers else { return };
        if !std::mem::replace(&mut self.written, true) {
            buffers.write(&self.lines, self.shape);
        }
        let (w, _) = size(self.shape, scale);
        let at = (width.saturating_sub(w) / 2, 24 * scale);
        buffers.copy(&list.list, back, at, (width, height));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every character the lines use has its own picture, and no two look
    /// alike.
    #[test]
    fn characters_differ() {
        let used: Vec<u8> = (b'0'..=b'9').chain(b'A'..=b'Z').chain(*b"./- ").collect();
        for (i, a) in used.iter().enumerate() {
            assert_ne!(glyph(*a), glyph(b'#'), "{} has a picture", *a as char);
            assert!(glyph(*a).iter().all(|row| row >> GLYPH.0 == 0));
            for b in &used[i + 1..] {
                assert_ne!(glyph(*a), glyph(*b), "{} {}", *a as char, *b as char);
            }
        }
    }

    /// The frame generation line fits the box and says what is shown.
    #[test]
    fn generated_frames_are_told() {
        assert_eq!(generated(27.5, 1), "FRAME GEN OFF");
        assert_eq!(generated(27.5, 6), "FRAME GEN 6X  SHOWN 165.0 FPS");
        assert!(generated(999.9, 6).len() <= COLUMNS as usize);
    }

    #[test]
    fn a_box_is_painted() {
        let lines = vec!["I".to_owned(), " i".to_owned()];
        for scale in [1, 2, 4] {
            let (width, height) = size(PANEL, scale);
            let pitch = (width * 4).next_multiple_of(256) as usize;
            let colours = colours(DXGI_FORMAT_R10G10B10A2_UNORM).unwrap();
            let picture = painted(&lines, PANEL, scale, colours, pitch);
            assert_eq!(picture.len(), pitch * height as usize);
            let at = |x: u32, y: u32| &picture[y as usize * pitch + x as usize * 4..][..4];
            // The rim is black, the first letter's top row lit in its middle.
            assert_eq!(at(0, 0), colours[0]);
            assert_eq!(at((RIM + 2) * scale, RIM * scale), colours[1]);
            assert_eq!(at((RIM + 2) * scale + scale - 1, RIM * scale + scale - 1), colours[1]);
            assert_eq!(at(RIM * scale, RIM * scale), colours[0]);
            // Small letters are shown as capitals, a line and a cell on.
            assert_eq!(at((RIM + CELL.0 + 2) * scale, (RIM + CELL.1) * scale), colours[1]);
            // Nothing past a line's end.
            assert_eq!(at((RIM + CELL.0 * 2 + 2) * scale, (RIM + CELL.1) * scale), colours[0]);
        }
    }

    /// Under Wine our own Linux process is found, by its command line, and
    /// its memory read; on Windows there is no `/proc`.
    #[test]
    fn our_linux_process_is_found() {
        if std::fs::metadata(r"Z:\proc").is_ok() {
            assert!(own_linux_process().is_some());
            assert!(linux_resident().is_some_and(|b| b > 1 << 20));
        } else {
            assert!(own_linux_process().is_none());
        }
    }

    #[test]
    fn counters_that_went_back_to_nothing_count_from_there() {
        let (mut a, mut b) = (Counters::default(), Counters::default());
        (a.0[0], b.0[0]) = (10, 30);
        (a.0[1], b.0[1]) = (50, 20);
        let c = b.since(a, 10.0);
        assert_eq!((c[0], c[1]), (2.0, 2.0));
    }
}
