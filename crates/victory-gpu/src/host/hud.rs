//! The run's numbers panel (`panel.rs`) as a picture: a canvas the CPU
//! paints (boxes, meters, graphs), text in the system's own fonts through
//! GDI, and at the end the swap chain's pixel format.
//!
//! No font ships with VIctory: on Windows the text is Segoe UI and Consolas;
//! under Wine, the Linux fonts fontconfig offers (Noto Sans, DejaVu Sans and
//! their monospaced kin), whichever is there first ([`SANS`], [`MONO`]).
//! What cannot be had (no GDI, no font at all) leaves the panel to its dot
//! letters.

use windows::Win32::Foundation::{COLORREF, SIZE};
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;

/// A colour, in sRGB.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) struct Rgb(pub u8, pub u8, pub u8);

/// The panel's colours.
pub(super) mod palette {
    use super::Rgb;
    pub(in crate::host) const BACK: Rgb = Rgb(0x0e, 0x12, 0x19);
    pub(in crate::host) const CARD: Rgb = Rgb(0x17, 0x1d, 0x27);
    pub(in crate::host) const EDGE: Rgb = Rgb(0x2b, 0x34, 0x44);
    pub(in crate::host) const GRID: Rgb = Rgb(0x30, 0x39, 0x4a);
    pub(in crate::host) const TEXT: Rgb = Rgb(0xee, 0xf2, 0xf7);
    pub(in crate::host) const DIM: Rgb = Rgb(0x8d, 0x98, 0xab);
    pub(in crate::host) const ACCENT: Rgb = Rgb(0x4f, 0xc3, 0xf7);
    pub(in crate::host) const VIOLET: Rgb = Rgb(0xb3, 0x88, 0xff);
    pub(in crate::host) const GOOD: Rgb = Rgb(0x5f, 0xd3, 0x7a);
    pub(in crate::host) const FAIR: Rgb = Rgb(0xff, 0xcc, 0x33);
    pub(in crate::host) const POOR: Rgb = Rgb(0xff, 0x8c, 0x3a);
    pub(in crate::host) const BAD: Rgb = Rgb(0xf2, 0x55, 0x52);
}

/// What a share of something (0 to 1) looks like: green to red as it fills.
pub(super) fn load_colour(share: f64) -> Rgb {
    match share {
        s if s < 0.6 => palette::GOOD,
        s if s < 0.85 => palette::FAIR,
        s if s < 0.95 => palette::POOR,
        _ => palette::BAD,
    }
}

/// What a frame of `ms` looks like: green at 60 a second, red under 15.
pub(super) fn frame_colour(ms: f64) -> Rgb {
    match ms {
        m if m <= 1000.0 / 55.0 => palette::GOOD,
        m if m <= 1000.0 / 28.0 => palette::FAIR,
        m if m <= 1000.0 / 15.0 => palette::POOR,
        _ => palette::BAD,
    }
}

/// How opaque the panel's background is over the title's picture, and its
/// cards (the graphs' and meters' grounds) over that. Text, bars and lines
/// are opaque. (Where the panel is copied rather than blended, all of it
/// is opaque.)
pub(super) const BACKGROUND: f32 = 0.33;
pub(super) const CARDS: f32 = 0.55;

/// Pixels the CPU paints: colour and how opaque (straight, not
/// premultiplied).
pub(super) struct Canvas {
    pub(super) width: u32,
    pub(super) height: u32,
    pixels: Vec<(Rgb, f32)>,
}

impl Canvas {
    pub(super) fn new(width: u32, height: u32, fill: Rgb, alpha: f32) -> Canvas {
        Canvas { width, height, pixels: vec![(fill, alpha); (width * height) as usize] }
    }

    pub(super) fn at(&self, x: u32, y: u32) -> Rgb {
        self.pixels[(y * self.width + x) as usize].0
    }

    #[cfg(test)]
    pub(super) fn alpha_at(&self, x: u32, y: u32) -> f32 {
        self.pixels[(y * self.width + x) as usize].1
    }

    /// `colour` over the pixel at `x, y` by `alpha` (0 to 1).
    fn blend(&mut self, x: i32, y: i32, colour: Rgb, alpha: f32) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height || alpha <= 0.0 {
            return;
        }
        let p = &mut self.pixels[(y as u32 * self.width + x as u32) as usize];
        if alpha >= 1.0 {
            *p = (colour, 1.0);
            return;
        }
        // Over, straight: the result's opacity, and its colour weighed by
        // what each side gives.
        let under = p.1 * (1.0 - alpha);
        let out = alpha + under;
        let mix = |a: u8, b: u8| ((f32::from(b) * alpha + f32::from(a) * under) / out).round() as u8;
        *p = (Rgb(mix(p.0.0, colour.0), mix(p.0.1, colour.1), mix(p.0.2, colour.2)), out);
    }

    pub(super) fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: Rgb, alpha: f32) {
        for j in y.max(0)..(y + h).min(self.height as i32) {
            for i in x.max(0)..(x + w).min(self.width as i32) {
                self.blend(i, j, colour, alpha);
            }
        }
    }

    /// A box with corners rounded by `r` pixels, their edge smoothed.
    pub(super) fn round_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, colour: Rgb) {
        self.round_rect_by(x, y, w, h, r, colour, 1.0);
    }

    /// [`Canvas::round_rect`] at `opacity`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn round_rect_by(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, colour: Rgb, opacity: f32) {
        let r = r.min(w / 2).min(h / 2).max(0);
        for j in 0..h {
            for i in 0..w {
                // Distance into the corner's circle, for the four corners.
                let cx = if i < r { r - i } else if i >= w - r { i - (w - r - 1) } else { 0 };
                let cy = if j < r { r - j } else if j >= h - r { j - (h - r - 1) } else { 0 };
                let alpha = if cx > 0 && cy > 0 {
                    let d = ((cx as f32 - 0.5).powi(2) + (cy as f32 - 0.5).powi(2)).sqrt();
                    (r as f32 - d + 0.5).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                self.blend(x + i, y + j, colour, alpha * opacity);
            }
        }
    }

    /// A meter: a rounded track and its share filled.
    pub(super) fn meter(&mut self, x: i32, y: i32, w: i32, h: i32, share: f64, colour: Rgb) {
        self.round_rect_by(x, y, w, h, h / 2, palette::GRID, 0.8);
        let fill = ((w as f64) * share.clamp(0.0, 1.0)).round() as i32;
        if fill > 0 {
            self.round_rect(x, y, fill.max(h), h, h / 2, colour);
        }
    }

    /// An area graph of `values` (oldest first) in a box, the newest at its
    /// right edge, `top` being its height's worth.
    pub(super) fn area(&mut self, x: i32, y: i32, w: i32, h: i32, values: &[f32], top: f32, colour: Rgb) {
        let n = values.len().min(w as usize);
        if n == 0 || top <= 0.0 {
            return;
        }
        let height = |v: f32| ((v / top).clamp(0.0, 1.0) * (h - 1) as f32).round() as i32;
        let step = w as f32 / n.max(2).saturating_sub(1) as f32;
        let at = |k: usize| x + w - 1 - (((n - 1 - k) as f32) * step).round() as i32;
        let shown = &values[values.len() - n..];
        for k in 0..n {
            let (x0, v0) = (at(k), height(shown[k]));
            let (x1, v1) = if k + 1 < n { (at(k + 1), height(shown[k + 1])) } else { (x0 + 1, v0) };
            for px in x0..x1.max(x0 + 1) {
                let t = if x1 > x0 { (px - x0) as f32 / (x1 - x0) as f32 } else { 0.0 };
                let v = (v0 as f32 + (v1 - v0) as f32 * t).round() as i32;
                let top_y = y + h - 1 - v;
                for py in top_y..y + h {
                    self.blend(px, py, colour, 0.22);
                }
                self.blend(px, top_y, colour, 1.0);
                self.blend(px, top_y - 1, colour, 0.5);
            }
        }
    }

    /// The canvas as rows of `pitch` bytes of RGBA, straight alpha: for the
    /// pass that blends it onto the picture.
    pub(super) fn rgba(&self, pitch: usize) -> Vec<u8> {
        let mut out = vec![0u8; pitch * self.height as usize];
        for y in 0..self.height {
            let row = &mut out[pitch * y as usize..][..self.width as usize * 4];
            for (x, px) in row.chunks_exact_mut(4).enumerate() {
                let (Rgb(r, g, b), a) = self.pixels[(y * self.width) as usize + x];
                px.copy_from_slice(&[r, g, b, (a * 255.0).round() as u8]);
            }
        }
        out
    }

    /// The canvas as rows of `pitch` bytes of `format`, opaque; `None` for a
    /// format not known here.
    pub(super) fn encode(&self, format: DXGI_FORMAT, pitch: usize) -> Option<Vec<u8>> {
        let size = match format {
            DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_B8G8R8A8_UNORM | DXGI_FORMAT_R10G10B10A2_UNORM => 4,
            DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
            _ => return None,
        };
        let mut out = vec![0u8; pitch * self.height as usize];
        for y in 0..self.height {
            let row = &mut out[pitch * y as usize..][..self.width as usize * size];
            for (x, px) in row.chunks_exact_mut(size).enumerate() {
                let Rgb(r, g, b) = self.at(x as u32, y);
                match format {
                    DXGI_FORMAT_R8G8B8A8_UNORM => px.copy_from_slice(&[r, g, b, 0xFF]),
                    DXGI_FORMAT_B8G8R8A8_UNORM => px.copy_from_slice(&[b, g, r, 0xFF]),
                    DXGI_FORMAT_R10G10B10A2_UNORM => {
                        let ten = |c: u8| (u32::from(c) * 1023 + 127) / 255;
                        let word = ten(r) | ten(g) << 10 | ten(b) << 20 | 3 << 30;
                        px.copy_from_slice(&word.to_le_bytes());
                    }
                    _ => {
                        // scRGB: linear light, 1 being SDR white.
                        for (i, c) in [r, g, b].into_iter().enumerate() {
                            px[2 * i..2 * i + 2].copy_from_slice(&half(linear(c)).to_le_bytes());
                        }
                        px[6..8].copy_from_slice(&half(1.0).to_le_bytes());
                    }
                }
            }
        }
        Some(out)
    }
}

/// An sRGB byte as linear light.
fn linear(c: u8) -> f32 {
    let c = f32::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// A float from 0 to 65504 as a half (no infinities or NaNs here).
fn half(v: f32) -> u16 {
    let v = v.clamp(0.0, 65504.0);
    if v < 6.103_515_6e-5 {
        // Subnormal: in steps of 2^-24.
        return (v * 16_777_216.0).round() as u16;
    }
    let bits = v.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mantissa = (bits & 0x7f_ffff) + 0x1000; // rounded to 10 bits
    let (exponent, mantissa) = if mantissa & 0x80_0000 != 0 { (exponent + 1, 0) } else { (exponent, mantissa >> 13) };
    ((exponent as u16) << 10) | mantissa as u16
}

/// Family names tried for labels, in order.
/// (Proton's Wine sees its own fonts: its Arial and Microsoft Sans Serif
/// stand-ins are sans.)
/// (Of those Linux has, DejaVu Sans is the most strongly hinted: grey
/// anti-aliased at panel sizes it is the sharpest, Noto Sans softer.)
const SANS: &[&str] = &[
    "Segoe UI",
    "DejaVu Sans",
    "Inter",
    "Noto Sans",
    "Cantarell",
    "Ubuntu",
    "DejaVu Sans",
    "Liberation Sans",
    "Arial",
    "Microsoft Sans Serif",
    "Tahoma",
];
/// And for numbers, which should not jump about as they change.
const MONO: &[&str] = &[
    "Cascadia Mono",
    "Consolas",
    "DejaVu Sans Mono",
    "JetBrains Mono",
    "Noto Sans Mono",
    "Ubuntu Mono",
    "Hack",
    "Liberation Mono",
    "Lucida Console",
    "Courier New",
];

/// A kind of text on the panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Style {
    /// The frame rate itself.
    Huge,
    /// Big numbers.
    Large,
    /// Values.
    Value,
    /// Section heads, small and spaced.
    Head,
    /// Labels and notes.
    Label,
    /// A graph's levels.
    Tick,
    /// The tape's words and its numbers, `step` lines above the newest:
    /// smaller the further up.
    Tape(u8),
    TapeMono(u8),
}

impl Style {
    /// Height in pixels at a scale of 1, weight, and whether it is monospaced.
    fn form(self) -> (f32, u32, bool) {
        match self {
            Style::Huge => (40.0, FW_BOLD.0, true),
            Style::Large => (19.0, FW_SEMIBOLD.0, true),
            Style::Value => (14.0, FW_SEMIBOLD.0, true),
            Style::Head => (11.0, FW_BOLD.0, false),
            Style::Label => (13.0, FW_MEDIUM.0, false),
            Style::Tick => (10.0, FW_SEMIBOLD.0, false),
            Style::Tape(step) => (tape_size(step), FW_MEDIUM.0, false),
            Style::TapeMono(step) => (tape_size(step), FW_MEDIUM.0, true),
        }
    }
}

/// A tape line's text height `step` lines above the newest, at a scale of 1.
fn tape_size(step: u8) -> f32 {
    14.0 * (1.0 - 0.09 * f32::from(step)).max(0.5)
}

/// How opaque a tape line `step` lines above the newest is.
fn tape_alpha(step: u8) -> f32 {
    (1.0 - 0.15 * f32::from(step)).max(0.12)
}

/// Where a text's `x` is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Align {
    Left,
    Right,
}

/// Text through GDI: a memory DC, a bitmap it draws into, and fonts.
pub(super) struct Type {
    dc: HDC,
    bitmap: HBITMAP,
    bits: *mut u32,
    size: (u32, u32),
    scale: f32,
    faces: [Vec<u16>; 2],
    fonts: Vec<(Style, HFONT)>,
    /// How GDI smooths the letters, and how their coverage is taken.
    quality: FONT_QUALITY,
}

// SAFETY: used by the presenting thread alone, under the presenter's lock.
unsafe impl Send for Type {}

/// `s` as a NUL-terminated UTF-16 string.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl Type {
    /// For a canvas `size` wide and texts at `scale`; `None` without GDI or
    /// a bitmap to draw into.
    pub(super) fn new(size: (u32, u32), scale: f32) -> Option<Type> {
        let header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size.0 as i32,
            biHeight: -(size.1 as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let info = BITMAPINFO { bmiHeader: header, ..Default::default() };
        let mut bits = std::ptr::null_mut();
        // SAFETY: plain GDI object creation; the bits are the bitmap's own.
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return None;
            }
            let Ok(bitmap) = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) else {
                let _ = DeleteDC(dc);
                return None;
            };
            if bits.is_null() {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(dc);
                return None;
            }
            SelectObject(dc, bitmap.into());
            SetBkMode(dc, TRANSPARENT);
            SetTextColor(dc, COLORREF(0x00ff_ffff));
            let mut t = Type {
                dc,
                bitmap,
                bits: bits.cast(),
                size,
                scale,
                faces: [Vec::new(), Vec::new()],
                fonts: Vec::new(),
                quality: ANTIALIASED_QUALITY,
            };
            t.faces = [t.pick(SANS, false), t.pick(MONO, true)];
            tracing::info!(
                sans = %String::from_utf16_lossy(&t.faces[0][..t.faces[0].len() - 1]),
                mono = %String::from_utf16_lossy(&t.faces[1][..t.faces[1].len() - 1]),
                "the numbers panel's fonts"
            );
            Some(t)
        }
    }

    /// The first of `families` the system has, else the last: GDI says
    /// which face it gave for a name, and the face's own family must be of
    /// the kind asked for (`mono`: fixed pitch; else not serif). Wine
    /// answers with the name asked for where it substitutes another font
    /// (Proton's prefix has "Segoe UI" drawn by a serif face).
    fn pick(&mut self, families: &[&str], mono: bool) -> Vec<u16> {
        for family in families {
            let name = wide(family);
            // SAFETY: a font made, selected, asked about and deleted.
            let got = unsafe {
                let font = CreateFontW(-16, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, ANTIALIASED_QUALITY, 0, PCWSTR(name.as_ptr()));
                if font.is_invalid() {
                    continue;
                }
                let old = SelectObject(self.dc, font.into());
                let mut face = [0u16; 64];
                let n = GetTextFaceW(self.dc, Some(&mut face));
                let mut metrics = TEXTMETRICW::default();
                let measured = GetTextMetricsW(self.dc, &mut metrics).as_bool();
                SelectObject(self.dc, old);
                let _ = DeleteObject(font.into());
                let pitch = metrics.tmPitchAndFamily.0;
                // (TMPF_FIXED_PITCH set means the pitch is variable.)
                let kind = measured
                    && if mono { pitch & TMPF_FIXED_PITCH.0 == 0 } else { pitch & 0xf0 != FF_ROMAN.0 };
                (String::from_utf16_lossy(&face[..(n.max(1) as usize - 1).min(63)]), kind)
            };
            if got.0.eq_ignore_ascii_case(family) && got.1 {
                return name;
            }
        }
        wide(families[families.len() - 1])
    }

    /// The font of `style`, made once.
    fn font(&mut self, style: Style) -> HFONT {
        if let Some(&(_, font)) = self.fonts.iter().find(|(s, _)| *s == style) {
            return font;
        }
        let (px, weight, mono) = style.form();
        let face = &self.faces[usize::from(mono)];
        // SAFETY: a font made from a NUL-terminated name we hold.
        let font = unsafe {
            CreateFontW(
                -((px * self.scale).round() as i32).max(6),
                0,
                0,
                0,
                weight as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                self.quality,
                0,
                PCWSTR(face.as_ptr()),
            )
        };
        self.fonts.push((style, font));
        font
    }

    /// Labels in `face` from here on, if the system has it (fonts made
    /// again); whether it does.
    #[cfg(test)]
    pub(super) fn set_sans(&mut self, face: &str) -> bool {
        let picked = self.pick(&[face, "\u{1}"], false);
        let found = String::from_utf16_lossy(&picked[..picked.len() - 1]) == face;
        if found {
            self.faces[0] = picked;
            self.set_quality(self.quality);
        }
        found
    }

    /// Letters smoothed as `quality` says from here on (fonts made again).
    #[cfg(test)]
    pub(super) fn set_quality(&mut self, quality: FONT_QUALITY) {
        // SAFETY: the fonts this made.
        unsafe {
            for (_, font) in self.fonts.drain(..) {
                let _ = DeleteObject(font.into());
            }
        }
        self.quality = quality;
    }

    /// The faces it found, for labels and for numbers.
    #[cfg(test)]
    pub(super) fn faces(&self) -> [String; 2] {
        self.faces.each_ref().map(|f| String::from_utf16_lossy(&f[..f.len().saturating_sub(1)]))
    }

    /// The scale its fonts are made for.
    pub(super) fn scale(&self) -> f32 {
        self.scale
    }

    /// The size of `s` in `style`.
    pub(super) fn measure(&mut self, s: &str, style: Style) -> (i32, i32) {
        let font = self.font(style);
        let text: Vec<u16> = s.encode_utf16().collect();
        let mut size = SIZE::default();
        // SAFETY: our DC and font; the text lives through the call.
        unsafe {
            SelectObject(self.dc, font.into());
            let _ = GetTextExtentPoint32W(self.dc, &text, &mut size);
        }
        (size.cx, size.cy)
    }

    /// Writes `s` onto `canvas` in `style` and `colour`, its top at `y` and
    /// its left or right edge at `x`. Returns its width.
    pub(super) fn write(&mut self, canvas: &mut Canvas, x: i32, y: i32, s: &str, style: Style, colour: Rgb, align: Align) -> i32 {
        self.write_faded(canvas, x, y, s, style, colour, align, 1.0)
    }

    /// [`Type::write`] at `opacity` (0 to 1).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_faded(&mut self, canvas: &mut Canvas, x: i32, y: i32, s: &str, style: Style, colour: Rgb, align: Align, opacity: f32) -> i32 {
        let (w, h) = self.measure(s, style);
        let (w, h) = (w.min(self.size.0 as i32), h.min(self.size.1 as i32));
        if w <= 0 || h <= 0 {
            return 0;
        }
        let left = match align {
            Align::Left => x,
            Align::Right => x - w,
        };
        let text: Vec<u16> = s.encode_utf16().collect();
        let stride = self.size.0 as usize;
        // SAFETY: the bitmap's own bits, `size` of them; GDI is done with them
        // after the flush.
        unsafe {
            let bits = std::slice::from_raw_parts_mut(self.bits, stride * self.size.1 as usize);
            for row in 0..h as usize {
                bits[row * stride..row * stride + w as usize].fill(0);
            }
            let _ = TextOutW(self.dc, 0, 0, &text);
            let _ = GdiFlush();
            for row in 0..h {
                for col in 0..w {
                    let p = bits[row as usize * stride + col as usize];
                    let (r, g, b) = ((p >> 16) & 0xff, (p >> 8) & 0xff, p & 0xff);
                    let cover = if self.quality == CLEARTYPE_QUALITY { (r + g + b) / 3 } else { r.max(g).max(b) };
                    if cover > 0 {
                        canvas.blend(left + col, y + row, colour, sharpened(cover) * opacity);
                    }
                }
            }
        }
        w
    }
}

/// Grey anti-aliased coverage (0 to 255) as drawn: light letters blended on
/// a dark ground in sRGB come out thinner and softer than they are, so the
/// coverage is raised (a gamma, as stem darkening does) and its edge made a
/// little steeper.
fn sharpened(cover: u32) -> f32 {
    let a = cover as f32 / 255.0;
    (a.powf(0.72) * 1.18 - 0.1).clamp(0.0, 1.0)
}

impl Drop for Type {
    fn drop(&mut self) {
        // SAFETY: the objects this made.
        unsafe {
            for (_, font) in self.fonts.drain(..) {
                let _ = DeleteObject(font.into());
            }
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

/// The panel's numbers at one moment.
#[derive(Clone, Debug, Default)]
pub(super) struct Snapshot {
    /// Frames a second, a frame's time, the longest frame's (ms).
    pub(super) fps: f64,
    pub(super) ms: f64,
    pub(super) worst: f64,
    /// The time of the slowest frame in a hundred (ms).
    pub(super) slow: f64,
    /// Recent frames' times (ms), and the frame rate's history, oldest first.
    pub(super) frames: Vec<f32>,
    pub(super) rates: Vec<f32>,
    /// Frames shown for each one made (frame generation), 1 without.
    pub(super) shown_per: u32,
    /// Each processor's share busy (0 to 1), all of them, and our process's
    /// cores' worth.
    pub(super) cores: Vec<f32>,
    pub(super) system: f64,
    pub(super) ours: f64,
    /// The card's share busy (0 to 1).
    pub(super) gpu: f64,
    /// Bytes: video memory used and its budget; the process's memory that
    /// cannot be given back (under Wine, Linux's resident anonymous and
    /// shared memory; on Windows, private bytes), its working set, and the
    /// PC's memory.
    pub(super) vram: (u64, u64),
    pub(super) ram: (u64, u64, u64),
    /// Per frame: draws, draws dropped, ms the submitting thread worked and
    /// waited for the card, MB to the card and of textures, ms copying back.
    pub(super) draws: f64,
    pub(super) dropped: f64,
    pub(super) submit: f64,
    pub(super) waits: f64,
    pub(super) up: f64,
    pub(super) textures: f64,
    pub(super) back: f64,
    /// Pipelines being made now.
    pub(super) making: u64,
    /// What the panel costs a frame (microseconds).
    pub(super) cost: f64,
    /// The tape: newest first, seconds ago, kind, text; and how many of
    /// each kind since the start (`crate::feed`).
    pub(super) tape: Vec<(f32, crate::feed::Kind, String)>,
    pub(super) counts: [u64; 5],
}

/// The panel's base sizes, at a scale of 1 (1080 lines).
const WIDTH: f32 = 380.0;
const PAD: f32 = 14.0;

/// The scale for a picture `height` lines high.
pub(super) fn scale(height: u32) -> f32 {
    (height as f32 / 1080.0).clamp(0.75, 2.5)
}

/// Sections' heights at a scale of 1: the head, the frame times, the frame
/// rate, the processors, the meters, the work, the tape, the foot.
const SECTIONS: [f32; 8] = [56.0, 108.0, 72.0, 66.0, 76.0, 86.0, 176.0, 18.0];
const GAP: f32 = 10.0;

/// The panel's size in pixels at `scale`.
pub(super) fn size(scale: f32) -> (u32, u32) {
    let height = 2.0 * PAD + SECTIONS.iter().sum::<f32>() + GAP * (SECTIONS.len() - 1) as f32;
    ((WIDTH * scale).round() as u32, (height * scale).round() as u32)
}

/// A level for a graph's top: the first of these at or above `v`.
fn ceiling(v: f32, steps: &[f32]) -> f32 {
    steps.iter().copied().find(|&s| s >= v).unwrap_or(v)
}

/// Bytes as GB.
fn gb(bytes: u64) -> f64 {
    bytes as f64 / (1u64 << 30) as f64
}

/// A chart's range, small, right of it at `right`: its top value level
/// with its top, 0 level with its bottom.
fn range_marks(t: &mut Type, c: &mut Canvas, right: i32, y: i32, h: i32, top: f32) {
    let (_, th) = t.measure("0", Style::Tick);
    t.write(c, right, y, &format!("{top:.0}"), Style::Tick, palette::DIM, Align::Right);
    t.write(c, right, y + h - th, "0", Style::Tick, palette::DIM, Align::Right);
}

/// The panel for `s`.
pub(super) fn paint(t: &mut Type, s: &Snapshot) -> Canvas {
    let k = t.scale;
    let px = |v: f32| (v * k).round() as i32;
    let (w, h) = size(k);
    let mut c = Canvas::new(w, h, palette::BACK, BACKGROUND);
    // A rim.
    c.rect(0, 0, w as i32, 1, palette::EDGE, 0.7);
    c.rect(0, h as i32 - 1, w as i32, 1, palette::EDGE, 0.7);
    c.rect(0, 0, 1, h as i32, palette::EDGE, 0.7);
    c.rect(w as i32 - 1, 0, 1, h as i32, palette::EDGE, 0.7);
    c.rect(0, 0, w as i32, px(3.0), palette::ACCENT, 1.0);
    let (left, right) = (px(PAD), w as i32 - px(PAD));
    let inner = right - left;
    let mut y = px(PAD);
    let heights: Vec<i32> = SECTIONS.iter().map(|&v| px(v)).collect();
    let gap = px(GAP);

    // The head: the frame rate, a frame's time, the slow frames.
    let rate_colour = frame_colour(s.ms);
    let wrote = t.write(&mut c, left, y - px(4.0), &format!("{:.1}", s.fps), Style::Huge, rate_colour, Align::Left);
    t.write(&mut c, left + wrote + px(6.0), y + px(22.0), "FPS", Style::Head, palette::DIM, Align::Left);
    t.write(&mut c, right, y, &format!("{:.1} ms", s.ms), Style::Large, palette::TEXT, Align::Right);
    // (Labels grey, values white, written from the right.)
    let mut x = right;
    let mut pairs = vec![("max", format!("{:.0} ms", s.worst))];
    if s.slow > 0.0 {
        pairs.push(("1% low", format!("{:.1}", 1000.0 / s.slow)));
    }
    for (label, value) in pairs {
        x -= t.write(&mut c, x, y + px(25.0), &value, Style::Value, palette::TEXT, Align::Right) + px(4.0);
        x -= t.write(&mut c, x, y + px(26.0), label, Style::Label, palette::DIM, Align::Right) + px(12.0);
    }
    if s.shown_per > 1 {
        let shown = format!("frame gen {}x   {:.0} shown", s.shown_per, s.fps * f64::from(s.shown_per));
        t.write(&mut c, right, y + px(40.0), &shown, Style::Label, palette::VIOLET, Align::Right);
    }
    y += heights[0] + gap;

    // Frame times, newest at the right; the chart's range, small, outside its
    // right edge (its top at the top, 0 at the bottom), its unit in the head.
    let gutter = px(30.0);
    let chart = inner - gutter;
    t.write(&mut c, left, y, "FRAME TIME · MS", Style::Head, palette::DIM, Align::Left);
    let mut sorted = s.frames.clone();
    sorted.sort_by(f32::total_cmp);
    let p95 = sorted.get(sorted.len() * 95 / 100).copied().unwrap_or(0.0);
    let top = ceiling(
        p95.max(20.0) * 1.15,
        &[25.0, 40.0, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 400.0, 500.0, 750.0, 1000.0, 1500.0, 2000.0, 3000.0, 5000.0],
    );
    t.write(&mut c, right, y, &format!("last {} frames", s.frames.len()), Style::Label, palette::DIM, Align::Right);
    let (gy, gh) = (y + px(18.0), heights[1] - px(18.0));
    c.round_rect_by(left, gy, chart, gh, px(5.0), palette::CARD, CARDS);
    let bar = px(3.0).max(2);
    let fit = (chart / (bar + 1)).max(1) as usize;
    let shown = &s.frames[s.frames.len().saturating_sub(fit)..];
    for (i, &ms) in shown.iter().enumerate() {
        let x = left + chart - (shown.len() - i) as i32 * (bar + 1);
        let bh = ((ms / top).clamp(0.0, 1.0) * (gh - px(4.0)) as f32).round().max(1.0) as i32;
        c.rect(x, gy + gh - px(2.0) - bh, bar, bh, frame_colour(f64::from(ms)), 1.0);
    }
    range_marks(t, &mut c, right, gy, gh, top);
    y += heights[1] + gap;

    // The frame rate over the last minute, from 0 to 60, the goal: more (a
    // loading screen's hundreds a second) is cut at the top. Its median,
    // which loading screens do not move.
    t.write(&mut c, left, y, "FPS · LAST MINUTE", Style::Head, palette::DIM, Align::Left);
    let mut rates = s.rates.clone();
    rates.sort_by(f32::total_cmp);
    let median = rates.get(rates.len() / 2).copied().unwrap_or(0.0);
    let top = 60.0;
    t.write(&mut c, right, y, &format!("median {median:.1}"), Style::Label, palette::DIM, Align::Right);
    let (gy, gh) = (y + px(18.0), heights[2] - px(18.0));
    c.round_rect_by(left, gy, chart, gh, px(5.0), palette::CARD, CARDS);
    c.area(left + px(2.0), gy + px(2.0), chart - px(4.0), gh - px(4.0), &s.rates, top, palette::ACCENT);
    range_marks(t, &mut c, right, gy, gh, top);
    y += heights[2] + gap;

    // Every processor.
    t.write(&mut c, left, y, &format!("CPU · {} THREADS", s.cores.len()), Style::Head, palette::DIM, Align::Left);
    let busy = format!("victory {:.1} cores   all {:.0}%", s.ours, s.system * 100.0);
    t.write(&mut c, right, y, &busy, Style::Label, palette::DIM, Align::Right);
    let (gy, gh) = (y + px(18.0), heights[3] - px(18.0));
    let n = s.cores.len().max(1) as i32;
    let space = px(2.0).max(1);
    let each = ((inner - space * (n - 1)) / n).max(1);
    let x0 = left + (inner - (each * n + space * (n - 1))) / 2;
    for (i, &share) in s.cores.iter().enumerate() {
        let x = x0 + i as i32 * (each + space);
        c.rect(x, gy, each, gh, palette::CARD, CARDS);
        let fill = ((f64::from(share)).clamp(0.0, 1.0) * f64::from(gh)).round() as i32;
        c.rect(x, gy + gh - fill, each, fill, load_colour(f64::from(share)), 1.0);
    }
    y += heights[3] + gap;

    // The card, its memory, the PC's memory.
    let row = heights[4] / 3;
    let (label_w, value_w) = (px(46.0), px(118.0));
    let meters = [
        ("GPU", s.gpu, format!("{:.0}% busy", s.gpu * 100.0)),
        ("VRAM", if s.vram.1 > 0 { s.vram.0 as f64 / s.vram.1 as f64 } else { 0.0 }, format!("{:.1} / {:.1} GB", gb(s.vram.0), gb(s.vram.1))),
        ("RAM", if s.ram.2 > 0 { s.ram.0 as f64 / s.ram.2 as f64 } else { 0.0 }, format!("{:.1} / {:.0} GB", gb(s.ram.0), gb(s.ram.2))),
    ];
    for (i, (name, share, value)) in meters.iter().enumerate() {
        let ry = y + i as i32 * row;
        t.write(&mut c, left, ry + px(3.0), name, Style::Head, palette::DIM, Align::Left);
        c.meter(left + label_w, ry + px(6.0), inner - label_w - value_w - px(8.0), px(9.0), *share, load_colour(*share));
        t.write(&mut c, right, ry + px(1.0), value, Style::Value, palette::TEXT, Align::Right);
    }
    y += heights[4] + gap;

    // The work of a frame, in two columns.
    t.write(&mut c, left, y, "PER FRAME", Style::Head, palette::DIM, Align::Left);
    let items = [
        ("draws", format!("{:.0}", s.draws), palette::TEXT),
        ("dropped", format!("{:.0}", s.dropped), if s.dropped >= 1.0 { palette::POOR } else { palette::TEXT }),
        ("pipelines building", format!("{}", s.making), if s.making > 0 { palette::FAIR } else { palette::TEXT }),
        ("submit busy", format!("{:.1} ms", s.submit), palette::TEXT),
        ("waits for GPU", format!("{:.1} ms", s.waits), palette::TEXT),
        ("to the card", format!("{:.0} MB", s.up), palette::TEXT),
        ("textures", format!("{:.0} MB", s.textures), palette::TEXT),
        ("copy back", format!("{:.1} ms", s.back), palette::TEXT),
    ];
    let column = (inner - px(16.0)) / 2;
    let line = (heights[5] - px(18.0)) / 4;
    for (i, (name, value, colour)) in items.iter().enumerate() {
        let (cx, cy) = (left + (i as i32 / 4) * (column + px(16.0)), y + px(18.0) + (i as i32 % 4) * line);
        t.write(&mut c, cx, cy, name, Style::Label, palette::DIM, Align::Left);
        t.write(&mut c, cx + column, cy, value, Style::Value, *colour, Align::Right);
    }
    y += heights[5] + gap;

    // The tape: the newest on the bottom line, full size; older ones above
    // it, smaller and fainter the further up, as on a rolling tape.
    t.write(&mut c, left, y, "SHADERS & PIPELINES", Style::Head, palette::DIM, Align::Left);
    let [translated, untranslated, made, refused, drawn] = s.counts;
    // (The totals on a row of their own, under the head, from the right.)
    let mut x = right;
    let y_totals = y + px(18.0);
    for (value, label, colour) in [
        (untranslated + refused, "failed", if untranslated + refused > 0 { palette::BAD } else { palette::TEXT }),
        (drawn, "drawn", palette::TEXT),
        (made, "made", palette::TEXT),
        (translated, "translated", palette::TEXT),
    ] {
        x -= t.write(&mut c, x, y_totals + px(1.0), label, Style::Label, palette::DIM, Align::Right) + px(4.0);
        x -= t.write(&mut c, x, y_totals, &value.to_string(), Style::Value, colour, Align::Right) + px(12.0);
    }
    let (tape_top, tape_bottom) = (y + px(40.0), y + heights[6]);
    c.round_rect_by(left, tape_top, inner, tape_bottom - tape_top, px(5.0), palette::CARD, CARDS);
    let mut line_bottom = tape_bottom - px(4.0);
    for (step, (ago, kind, text)) in s.tape.iter().enumerate() {
        let step = step.min(255) as u8;
        let (_, lh) = t.measure("Ag", Style::Tape(step));
        let top = line_bottom - lh;
        if top < tape_top + px(2.0) {
            break;
        }
        let opacity = tape_alpha(step);
        let (word, colour) = match kind {
            crate::feed::Kind::Translated => ("translated", palette::ACCENT),
            crate::feed::Kind::Untranslated => ("no translation", palette::BAD),
            crate::feed::Kind::Made => ("pipeline made", palette::GOOD),
            crate::feed::Kind::Refused => ("pipeline refused", palette::POOR),
            crate::feed::Kind::Drawn => ("first drawn", palette::VIOLET),
        };
        let size = tape_size(step) / tape_size(0);
        let dot = ((6.0 * k * size).round() as i32).max(3);
        let cx = left + px(8.0);
        // (A faded dot: blended toward the card.)
        let mix = |a: u8, b: u8| (f32::from(b) + (f32::from(a) - f32::from(b)) * opacity).round() as u8;
        let dot_colour = Rgb(mix(colour.0, palette::CARD.0), mix(colour.1, palette::CARD.1), mix(colour.2, palette::CARD.2));
        c.round_rect(cx, top + (lh - dot) / 2, dot, dot, dot / 2, dot_colour);
        let wx = cx + dot + px(6.0);
        let ww = t.write_faded(&mut c, wx, top, word, Style::Tape(step), colour, Align::Left, opacity);
        let age = if *ago < 60.0 { format!("{ago:.0} s") } else { format!("{:.0} min", ago / 60.0) };
        let aw = t.write_faded(&mut c, right - px(8.0), top, &age, Style::TapeMono(step), palette::DIM, Align::Right, opacity);
        // The text, cut to what fits between the word and the age.
        let room = right - px(8.0) - aw - px(10.0) - (wx + ww + px(8.0));
        let mut shown = text.clone();
        while !shown.is_empty() && t.measure(&shown, Style::TapeMono(step)).0 > room {
            shown.pop();
        }
        if shown.len() < text.len() && shown.len() > 1 {
            shown.pop();
            shown.push('…');
        }
        t.write_faded(&mut c, wx + ww + px(8.0), top, &shown, Style::TapeMono(step), palette::TEXT, Align::Left, opacity);
        line_bottom = top - px(2.0);
    }
    y += heights[6] + gap;

    // The foot: the name at the left, the key in the middle (it stays put as
    // the numbers change), the panel's cost at the right.
    t.write(&mut c, left, y, "VIctoryNVIDIALINUX", Style::Label, palette::ACCENT, Align::Left);
    let (kw, _) = t.measure("F3 hide", Style::Label);
    t.write(&mut c, (w as i32 - kw) / 2, y, "F3 hide", Style::Label, palette::DIM, Align::Left);
    let cw = t.write(&mut c, right, y, "µs a frame", Style::Label, palette::DIM, Align::Right);
    t.write(&mut c, right - cw - px(4.0), y - px(1.0), &format!("{:.0}", s.cost), Style::Value, palette::TEXT, Align::Right);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Over a translucent ground, opaque paint is opaque and the ground's
    /// colour stays where nothing was painted; half paint is between.
    #[test]
    fn translucent_paint_adds_up() {
        let mut c = Canvas::new(4, 1, palette::BACK, BACKGROUND);
        c.rect(0, 0, 1, 1, palette::TEXT, 1.0);
        c.rect(1, 0, 1, 1, palette::TEXT, 0.5);
        assert_eq!((c.at(0, 0), c.alpha_at(0, 0)), (palette::TEXT, 1.0));
        assert!((c.alpha_at(1, 0) - (0.5 + BACKGROUND * 0.5)).abs() < 1e-6);
        assert!(c.at(1, 0).0 > palette::BACK.0 && c.at(1, 0).0 < palette::TEXT.0);
        assert_eq!((c.at(3, 0), c.alpha_at(3, 0)), (palette::BACK, BACKGROUND));
        let rgba = c.rgba(16);
        assert_eq!(&rgba[0..4], &[palette::TEXT.0, palette::TEXT.1, palette::TEXT.2, 255]);
        assert_eq!(rgba[15], (BACKGROUND * 255.0).round() as u8);
    }

    /// Halves of the values the panel uses come out as the format has them.
    #[test]
    fn halves_are_right() {
        assert_eq!(half(0.0), 0);
        assert_eq!(half(1.0), 0x3c00);
        assert_eq!(half(0.5), 0x3800);
        assert_eq!(half(2.0), 0x4000);
        assert_eq!(half(0.333_333_34), 0x3555);
        assert!(half(linear(1)) > 0);
    }

    /// Each format's pixels: the canvas's colour, opaque, in the format's
    /// order, at the pitch asked for.
    #[test]
    fn a_canvas_is_encoded() {
        let mut c = Canvas::new(3, 2, palette::BACK, 1.0);
        c.rect(1, 1, 1, 1, Rgb(0xff, 0x80, 0x00), 1.0);
        let rgba = c.encode(DXGI_FORMAT_R8G8B8A8_UNORM, 256).unwrap();
        assert_eq!(rgba.len(), 512);
        assert_eq!(&rgba[256 + 4..256 + 8], &[0xff, 0x80, 0x00, 0xff]);
        let bgra = c.encode(DXGI_FORMAT_B8G8R8A8_UNORM, 256).unwrap();
        assert_eq!(&bgra[256 + 4..256 + 8], &[0x00, 0x80, 0xff, 0xff]);
        let ten = c.encode(DXGI_FORMAT_R10G10B10A2_UNORM, 256).unwrap();
        let word = u32::from_le_bytes(ten[256 + 4..256 + 8].try_into().unwrap());
        assert_eq!((word & 0x3ff, word >> 30), (1023, 3));
        let float = c.encode(DXGI_FORMAT_R16G16B16A16_FLOAT, 256).unwrap();
        assert_eq!(&float[256 + 8..256 + 10], &half(1.0).to_le_bytes());
        assert_eq!(&float[256 + 14..256 + 16], &half(1.0).to_le_bytes());
        assert!(c.encode(DXGI_FORMAT_R32G32B32A32_FLOAT, 256).is_none());
    }

    /// Meters fill their share; area graphs stay in their box.
    #[test]
    fn meters_and_graphs_stay_in_their_place() {
        let mut c = Canvas::new(100, 40, palette::BACK, 1.0);
        c.meter(0, 0, 100, 10, 0.5, palette::GOOD);
        assert_eq!(c.at(20, 5), palette::GOOD);
        assert_ne!(c.at(80, 5), palette::GOOD);
        c.area(0, 20, 100, 20, &[1.0, 5.0, 10.0, 20.0], 10.0, palette::ACCENT);
        assert_eq!(c.at(99, 20), palette::ACCENT);
        assert_eq!(c.at(50, 10), palette::BACK);
    }

    /// Text in each smoothing GDI offers, to compare: with
    /// `VICTORY_HUD_PICTURE`, saved side by side as `hud-text.ppm`.
    #[test]
    fn text_in_each_smoothing() {
        let Some(folder) = std::env::var_os("VICTORY_HUD_PICTURE") else { return };
        let k = 1.33;
        let Some(mut t) = Type::new((420, 64), k) else { return };
        let mut c = Canvas::new(420, 3 * 44, palette::BACK, 1.0);
        for (i, q) in [ANTIALIASED_QUALITY, CLEARTYPE_QUALITY, NONANTIALIASED_QUALITY].into_iter().enumerate() {
            t.set_quality(q);
            let y = i as i32 * 44 + 4;
            t.write(&mut c, 6, y, "1% low", Style::Label, palette::DIM, Align::Left);
            t.write(&mut c, 70, y - 1, "5.3", Style::Value, palette::TEXT, Align::Left);
            t.write(&mut c, 120, y, "FRAME TIME  pipelines building", Style::Label, palette::TEXT, Align::Left);
            t.write(&mut c, 6, y + 20, "first drawn  mesh 6c3abd40 + pixel 77b790d0", Style::TapeMono(0), palette::TEXT, Align::Left);
        }
        let mut ppm = format!("P6 {} {} 255\n", c.width, c.height).into_bytes();
        for y in 0..c.height {
            for x in 0..c.width {
                let Rgb(r, g, b) = c.at(x, y);
                ppm.extend([r, g, b]);
            }
        }
        let _ = std::fs::write(std::path::Path::new(&folder).join("hud-text.ppm"), ppm);
    }

    /// The labels' line in each sans face there may be, grey anti-aliased,
    /// to compare their hinting: saved as `hud-faces.ppm`.
    #[test]
    fn text_in_each_face() {
        let Some(folder) = std::env::var_os("VICTORY_HUD_PICTURE") else { return };
        let Some(mut t) = Type::new((420, 64), 1.33) else { return };
        let faces = ["Noto Sans", "DejaVu Sans", "Liberation Sans", "Arial", "Cantarell", "Microsoft Sans Serif"];
        let mut c = Canvas::new(420, faces.len() as u32 * 24, palette::BACK, 1.0);
        let mut row = 0;
        for face in faces {
            if !t.set_sans(face) {
                continue;
            }
            let y = row * 24 + 3;
            t.write(&mut c, 6, y, "pipelines building  1% low", Style::Label, palette::TEXT, Align::Left);
            t.write(&mut c, 410, y, face, Style::Label, palette::DIM, Align::Right);
            row += 1;
        }
        let mut ppm = format!("P6 {} {} 255\n", c.width, c.height).into_bytes();
        for y in 0..c.height {
            for x in 0..c.width {
                let Rgb(r, g, b) = c.at(x, y);
                ppm.extend([r, g, b]);
            }
        }
        let _ = std::fs::write(std::path::Path::new(&folder).join("hud-faces.ppm"), ppm);
    }

    /// The whole panel paints at every scale it may be shown at, text and
    /// all, where the system has GDI (Windows, Wine).
    #[test]
    fn the_panel_paints() {
        for k in [0.75f32, 1.0, 1.33, 2.0] {
            let Some(mut t) = Type::new((size(k).0, (64.0 * k) as u32), k) else { return };
            let s = Snapshot {
                fps: 12.3,
                ms: 81.3,
                worst: 240.0,
                slow: 190.0,
                frames: (0..300).map(|i| 40.0 + (i % 50) as f32 * 3.0).collect(),
                rates: (0..120).map(|i| 5.0 + (i % 20) as f32).collect(),
                shown_per: 2,
                cores: vec![0.2, 0.5, 0.9, 1.0, 0.0, 0.3, 0.7, 0.1],
                system: 0.45,
                ours: 2.1,
                gpu: 0.37,
                vram: (11 << 30, 12 << 30),
                ram: (13 << 30, 14 << 30, 32 << 30),
                draws: 2440.0,
                dropped: 300.0,
                making: 12,
                tape: (0..12)
                    .map(|i| {
                        let kind = [crate::feed::Kind::Drawn, crate::feed::Kind::Made, crate::feed::Kind::Translated, crate::feed::Kind::Untranslated][i % 4];
                        (i as f32 * 1.7, kind, format!("mesh 6c3abd4{i} + pixel 77b790d{i}"))
                    })
                    .collect(),
                counts: [2533, 1, 2185, 0, 1840],
                ..Default::default()
            };
            let c = paint(&mut t, &s);
            assert_eq!((c.width, c.height), size(k));
            // To look at it: `VICTORY_HUD_PICTURE=<folder>` saves each as a PPM.
            if let Some(folder) = std::env::var_os("VICTORY_HUD_PICTURE") {
                eprintln!("faces: {:?}", t.faces());
                let mut ppm = format!("P6 {} {} 255\n", c.width, c.height).into_bytes();
                for y in 0..c.height {
                    for x in 0..c.width {
                        let Rgb(r, g, b) = c.at(x, y);
                        ppm.extend([r, g, b]);
                    }
                }
                let _ = std::fs::write(std::path::Path::new(&folder).join(format!("hud-{}.ppm", (k * 100.0) as u32)), ppm);
                let rgba = c.rgba(c.width as usize * 4);
                let _ = std::fs::write(std::path::Path::new(&folder).join(format!("hud-{}.rgba", (k * 100.0) as u32)), rgba);
            }
            // The accent stripe at the top, and something written: not all
            // of the head is background.
            assert_eq!(c.at(c.width / 2, 1), palette::ACCENT);
            let head = (0..c.width).flat_map(|x| (8..40).map(move |y| (x, y)));
            assert!(head.filter(|&(x, y)| y < c.height && c.at(x, y) != palette::BACK).count() > 50);
        }
    }
}
