//! The furrow overlay: oriented plough rows painted over ploughed squares.
//!
//! Ground textures tile without rotation, so a field's furrow direction
//! cannot live in the texture. The sidecar records it per square, and this
//! pass darkens the render in rows across it, in global coordinates so the
//! rows run on unbroken across windows. Rows fade out over the last square
//! of a field and stop under anything standing on it. Past the window edge
//! the edge square's field is taken to continue, so rows run on into the
//! neighbouring window instead of fading at the seam.

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
    let weight = |x: i64, y: i64| furrow_weight(side, &blocked, (w, h), x, y);
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

/// Furrow weight of square `(x, y)` of a `w × h` window: 1 on an unblocked
/// furrowed square, else 0. Squares past the window edge take the edge
/// square's weight (review round 2 #38: they counted as unfurrowed, so rows
/// faded over the last half square at every window edge).
fn furrow_weight(side: &Sidecar, blocked: &[bool], (w, h): (usize, usize), x: i64, y: i64) -> f64 {
    if w == 0 || h == 0 {
        return 0.0;
    }
    let clamp = |v: i64, n: usize| usize::try_from(v.max(0)).map_or(n - 1, |v| v.min(n - 1));
    let (x, y) = (clamp(x, w), clamp(y, h));
    let i = y * w + x;
    let furrowed = side.squares.get(i).is_some_and(|q| q.furrow.is_some());
    if furrowed && !blocked.get(i).copied().unwrap_or(true) {
        1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::{SquareRules, SrdCover, SIDECAR_VERSION};

    fn square(furrow: bool) -> SquareRules {
        SquareRules {
            ground: "ploughed".into(),
            difficult: true,
            water_depth_ft: 0,
            cover: SrdCover::None,
            blocks_sight: false,
            blocks_movement: false,
            deck: false,
            crop: None,
            furrow: furrow.then_some([1.0, 0.0]),
            field: None,
        }
    }

    #[test]
    fn furrows_run_on_past_the_window_edge() {
        // A 3 × 2 window: furrowed but for its east column.
        let side = Sidecar {
            format_version: SIDECAR_VERSION,
            name: "t".into(),
            seed: "1".into(),
            width: 3,
            height: 2,
            origin_square: [0, 0],
            square_m: 1.5625,
            squares: (0..6).map(|i| square(i % 3 != 2)).collect(),
            edges: Vec::new(),
            fields: Vec::new(),
            compounds: Vec::new(),
        };
        let blocked = vec![false; 6];
        let wt = |x, y| furrow_weight(&side, &blocked, (3, 2), x, y);
        // West and north of the window, the edge squares' field continues.
        assert!((wt(-1, 0) - 1.0).abs() < f64::EPSILON);
        assert!((wt(0, -1) - 1.0).abs() < f64::EPSILON);
        assert!((wt(-3, 5) - 1.0).abs() < f64::EPSILON);
        // East of it, the unfurrowed edge column continues unfurrowed.
        assert!(wt(3, 1).abs() < f64::EPSILON);
        assert!(wt(2, 0).abs() < f64::EPSILON);
    }
}
