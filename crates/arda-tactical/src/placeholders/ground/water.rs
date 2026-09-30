//! Water textures. Shallow water shows its sandy bottom through a teal
//! tint, with caustic light lines; deep water is a dark blue-green with
//! broad depth variation. Both carry ripple highlights. Depth gradients and
//! edge foam across a whole river are added by the compositor, which knows
//! where the banks are.

use super::kit::{base, spread, Palette};
use super::Tex;
use crate::placeholders::brush::Tile;
use crate::placeholders::material::Rng;
use crate::placeholders::paint::Rgb;

/// Ripple highlights: short, slightly curved strokes, mostly along x.
fn ripples(tile: &mut Tile, t: &Tex, rng: &mut Rng, n: u32, c: Rgb, opacity: (f32, f32)) {
    for _ in 0..t.n(n) {
        let a = (rng.range(0.0, t.size), rng.range(0.0, t.size));
        let l = rng.range(4.0, 12.0) * t.k;
        let bend = rng.range(-1.5, 1.5) * t.k;
        let dy = rng.range(-0.25, 0.25) * l;
        let m = (a.0 + l * 0.5, a.1 + dy * 0.5 + bend);
        let b = (a.0 + l, a.1 + dy);
        let w = rng.range(0.5, 1.0) * t.k;
        let o = rng.range(opacity.0, opacity.1);
        tile.stroke(a, m, w * 0.4, w, c, o);
        tile.stroke(m, b, w, w * 0.3, c, o);
    }
}

/// Shallow water over a sandy bottom.
pub fn shallow(t: &Tex) -> Tile {
    const P: Palette = [[60, 106, 102], [82, 128, 120], [112, 154, 140]];
    let mut tile = base(t, &P, 3);
    tile.map(|x, y, c| {
        let bottom = spread(t.noise(11, x, y, 4, 3), 2.0);
        let sand = [128.0, 138.0, 104.0];
        let k = 0.28 * bottom;
        let mut c = [
            c[0] + (sand[0] - c[0]) * k,
            c[1] + (sand[1] - c[1]) * k,
            c[2] + (sand[2] - c[2]) * k,
        ];
        let (f1, f2, _, _) = t.voronoi(x, y, 9, 0.9);
        let _ = f1;
        let edge = f2 - f1;
        if edge < 2.4 * t.k {
            let g = (1.0 - edge / (2.4 * t.k)) * 0.35 * t.noise(12, x, y, 6, 2).max(0.3);
            c = [
                c[0] + (200.0 - c[0]) * g,
                c[1] + (232.0 - c[1]) * g,
                c[2] + (218.0 - c[2]) * g,
            ];
        }
        c
    });
    let mut rng = t.rng(0x5A11);
    ripples(&mut tile, t, &mut rng, 70, [176, 214, 204], (0.3, 0.6));
    ripples(&mut tile, t, &mut rng, 30, [226, 240, 234], (0.5, 0.8));
    tile
}

/// Deep water: dark and cool with broad depth patches and ripples.
pub fn deep(t: &Tex) -> Tile {
    const P: Palette = [[22, 56, 76], [34, 80, 98], [56, 108, 122]];
    let mut tile = base(t, &P, 2);
    tile.map(|x, y, c| {
        let swell = t.noise(21, x + 20.0 * t.noise(22, x, y, 2, 2), y * 3.0, 6, 2);
        let k = 0.9 + 0.2 * swell;
        [c[0] * k, c[1] * k, c[2] * k]
    });
    let mut rng = t.rng(0xDEE9);
    ripples(&mut tile, t, &mut rng, 60, [74, 128, 142], (0.4, 0.7));
    ripples(&mut tile, t, &mut rng, 26, [150, 196, 204], (0.4, 0.7));
    tile
}
