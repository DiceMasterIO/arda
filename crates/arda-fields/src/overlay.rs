//! The furrow overlay: oriented plough rows painted over ploughed squares.
//!
//! Ground textures tile without rotation, so a field's furrow direction
//! cannot live in the texture. The sidecar records it per square, and this
//! pass darkens the render in rows across it, in global coordinates so the
//! rows run on unbroken across windows. Rows fade out over the last square
//! of a field and stop under anything standing on it.

use crate::sidecar::Sidecar;
use arda_tactical::compose::candidates;
use arda_tactical::layout::TacticalLayout;
use arda_tactical::raster::Rgba;
use arda_tactical::Library;

/// Plough-row spacing in squares (about 0.9 m).
pub const ROW_SQ: f64 = 0.58;

/// Paints furrows onto a render of `layout` at `ppsq` pixels per square.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn paint_furrows(
    img: &mut Rgba,
    layout: &TacticalLayout,
    side: &Sidecar,
    lib: &Library,
    ppsq: u32,
) {
    let (w, h) = (layout.width as usize, layout.height as usize);
    if side.squares.len() != w * h || img.width != layout.width * ppsq {
        return;
    }
    let mut blocked = vec![false; w * h];
    for p in &layout.placements {
        let all = candidates(lib, &p.asset);
        let (fw, fh) = all.iter().fold((0, 0), |(a, b), x| {
            (a.max(x.footprint.w), b.max(x.footprint.h))
        });
        let r = f64::from(fw.max(fh)) / 2.0;
        let (px, py) = (f64::from(p.x), f64::from(p.y));
        for y in ((py - r).floor().max(0.0) as usize)..((py + r).ceil().max(0.0) as usize).min(h) {
            for x in
                ((px - r).floor().max(0.0) as usize)..((px + r).ceil().max(0.0) as usize).min(w)
            {
                blocked[y * w + x] = true;
            }
        }
    }
    let weight = |x: i64, y: i64| -> f64 {
        if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
            return 0.0;
        }
        let i = y as usize * w + x as usize;
        if side.squares[i].furrow.is_some() && !blocked[i] {
            1.0
        } else {
            0.0
        }
    };
    let s = f64::from(ppsq);
    let (ox, oy) = (side.origin_square[0] as f64, side.origin_square[1] as f64);
    for py in 0..img.height {
        for px in 0..img.width {
            let (lx, ly) = ((f64::from(px) + 0.5) / s, (f64::from(py) + 0.5) / s);
            let (sx, sy) = (lx.floor() as usize, ly.floor() as usize);
            let Some(dir) = side.squares.get(sy * w + sx).and_then(|q| q.furrow) else {
                continue;
            };
            // Bilinear fade between square centres.
            let (fx, fy) = (lx - 0.5, ly - 0.5);
            let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
            let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
            let top = weight(ix, iy) * (1.0 - tx) + weight(ix + 1, iy) * tx;
            let bot = weight(ix, iy + 1) * (1.0 - tx) + weight(ix + 1, iy + 1) * tx;
            let fade = (top * (1.0 - ty) + bot * ty).clamp(0.0, 1.0);
            if fade <= 0.0 {
                continue;
            }
            let (ax, ay) = (f64::from(dir[0]), f64::from(dir[1]));
            let across = (ox + lx) * -ay + (oy + ly) * ax;
            let t = (across / ROW_SQ).rem_euclid(1.0);
            let ridge = (2.0 * t - 1.0).abs();
            let k = 1.0 + fade * (0.16 - 0.34 * (1.0 - ridge).powi(2));
            let mut c = img.get(px, py);
            for ch in c.iter_mut().take(3) {
                *ch = (f64::from(*ch) * k).round().clamp(0.0, 255.0) as u8;
            }
            img.set(px, py, c);
        }
    }
}
