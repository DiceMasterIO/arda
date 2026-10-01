//! A contact sheet for human curation (goal 62): every processed asset as
//! a labelled thumbnail. Cut-outs sit on a checkerboard so haze and halos
//! show; textures are tiled 2 × 2 so seams show. The frame is green when
//! clean, amber when flagged and red when rejected.

// Sheet geometry is small and bounded; casts are exact.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::font;
use crate::ops::resize;
use crate::report::{AssetReport, Status};
use arda_tactical::Rgba;

const THUMB: u32 = 192;
const PAD: u32 = 10;
const FRAME: u32 = 3;
const LABEL_SCALE: u32 = 2;
const COLUMNS: u32 = 8;
const BACKDROP: [u8; 4] = [38, 38, 42, 255];

/// One thumbnail's inputs.
#[derive(Debug, Clone, Copy)]
pub struct Entry<'a> {
    /// The processed image.
    pub img: &'a Rgba,
    /// Whether it is a tiling texture.
    pub texture: bool,
    /// The asset's report (id, status, flags).
    pub report: &'a AssetReport,
}

fn frame_colour(r: &AssetReport) -> [u8; 4] {
    match (r.status, r.flags.is_empty()) {
        (Status::Rejected, _) => [229, 57, 53, 255],
        (Status::Imported, false) => [255, 179, 0, 255],
        (Status::Imported, true) => [76, 175, 80, 255],
    }
}

fn checker(x: u32, y: u32) -> [u8; 4] {
    if (x / 12 + y / 12).is_multiple_of(2) {
        [200, 200, 205, 255]
    } else {
        [150, 150, 158, 255]
    }
}

fn tiled(img: &Rgba) -> Rgba {
    let mut out = Rgba::new(img.width * 2, img.height * 2);
    for y in 0..out.height {
        for x in 0..out.width {
            out.set(x, y, img.get(x % img.width, y % img.height));
        }
    }
    out
}

fn thumbnail(e: &Entry<'_>) -> Rgba {
    let src = if e.texture {
        tiled(e.img)
    } else {
        e.img.clone()
    };
    let scale = (THUMB as f32 / src.width as f32).min(THUMB as f32 / src.height as f32);
    let (w, h) = (
        ((src.width as f32 * scale).round() as u32).clamp(1, THUMB),
        ((src.height as f32 * scale).round() as u32).clamp(1, THUMB),
    );
    let small = resize(&src, w, h, e.texture);
    let mut out = Rgba::new(THUMB, THUMB);
    for y in 0..THUMB {
        for x in 0..THUMB {
            out.set(x, y, checker(x, y));
        }
    }
    let (ox, oy) = ((THUMB - w) / 2, (THUMB - h) / 2);
    for y in 0..h {
        for x in 0..w {
            out.blend(ox + x, oy + y, small.get(x, y));
        }
    }
    out
}

/// Splits an id into at most two label lines that fit the thumbnail,
/// breaking after a `.` or `_` where possible.
fn label_lines(id: &str) -> Vec<String> {
    let per = ((THUMB / font::advance(LABEL_SCALE)) as usize).max(1);
    let chars: Vec<char> = id.chars().collect();
    if chars.len() <= per {
        return vec![id.to_string()];
    }
    let cut = (1..=per)
        .rev()
        .find(|&k| matches!(chars[k - 1], '.' | '_'))
        .unwrap_or(per);
    let first: String = chars[..cut].iter().collect();
    let rest: String = chars[cut..].iter().take(per).collect();
    vec![first, rest]
}

/// Lays out the sheet.
#[must_use]
pub fn sheet(entries: &[Entry<'_>]) -> Rgba {
    let n = entries.len().max(1) as u32;
    let cols = n.min(COLUMNS);
    let rows = n.div_ceil(cols);
    let label_h = 2 * font::line_height(LABEL_SCALE);
    let (cw, ch) = (THUMB + 2 * PAD, THUMB + 2 * PAD + label_h);
    let mut img = Rgba::filled(cols * cw, rows * ch, BACKDROP);
    for (k, e) in entries.iter().enumerate() {
        let (cx, cy) = ((k as u32 % cols) * cw, (k as u32 / cols) * ch);
        let colour = frame_colour(e.report);
        for y in cy + PAD - FRAME..cy + PAD + THUMB + FRAME {
            for x in cx + PAD - FRAME..cx + PAD + THUMB + FRAME {
                img.set(x, y, colour);
            }
        }
        let thumb = thumbnail(e);
        for y in 0..THUMB {
            for x in 0..THUMB {
                img.set(cx + PAD + x, cy + PAD + y, thumb.get(x, y));
            }
        }
        for (line, text) in label_lines(&e.report.id).iter().enumerate() {
            let y = cy + PAD + THUMB + FRAME + 3 + line as u32 * font::line_height(LABEL_SCALE);
            font::draw(
                &mut img,
                cx + PAD,
                y,
                text,
                LABEL_SCALE,
                [225, 225, 230, 255],
            );
        }
    }
    img
}
