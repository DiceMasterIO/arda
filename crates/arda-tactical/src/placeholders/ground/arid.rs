//! Arid-basin floors (recipe 7 playas): white salt crust with raised
//! polygon ridges, and pale mudflat clay split by desiccation cracks.
//! Temporary generated art, like the rest of the placeholder library.

use super::kit::{anywhere, base, dabs, Palette};
use super::Tex;
use crate::placeholders::brush::Tile;

/// Salt crust: bright evaporite in polygons whose rims are pushed up into
/// low, slightly grey ridges, with a faint buff dust between.
pub fn salt_crust(t: &Tex) -> Tile {
    const P: Palette = [[214, 208, 196], [232, 228, 218], [246, 244, 238]];
    let mut tile = base(t, &P, 3);
    tile.map(|x, y, c| {
        let (f1, f2, id, _) = t.voronoi(x, y, 6, 0.85);
        let edge = f2 - f1;
        // Each polygon a touch brighter or duller; rims ridge and shade.
        let tint = 0.97 + 0.05 * ((id % 97) as f32 / 97.0);
        let mut c = [c[0] * tint, c[1] * tint, c[2] * tint];
        if edge < 1.4 * t.k {
            c = [c[0] * 0.8, c[1] * 0.79, c[2] * 0.77];
        } else if edge < 3.2 * t.k {
            c = [
                (c[0] * 1.04).min(255.0),
                (c[1] * 1.04).min(255.0),
                (c[2] * 1.04).min(255.0),
            ];
        }
        c
    });
    let mut rng = t.rng(0x5A17);
    dabs(
        &mut tile,
        t,
        &mut rng,
        50,
        (4.0, 12.0),
        &[[204, 194, 172], [220, 212, 194], [236, 230, 216]],
        (0.15, 0.35),
        &anywhere,
    );
    tile
}

/// Mudflat: pale buff clay curled into plates by thin dark cracks, with a
/// few damp darker patches.
pub fn mudflat(t: &Tex) -> Tile {
    const P: Palette = [[170, 150, 118], [192, 174, 142], [210, 194, 164]];
    let mut tile = base(t, &P, 4);
    tile.map(|x, y, c| {
        let (f1, f2, _, _) = t.voronoi(x, y, 9, 0.9);
        let edge = f2 - f1;
        let damp = t.noise(0x3D, x, y, 3, 2);
        let k = if damp > 0.55 { 0.9 } else { 1.0 };
        if edge < 1.1 * t.k {
            [c[0] * 0.55, c[1] * 0.52, c[2] * 0.48]
        } else if edge < 2.4 * t.k {
            [c[0] * 0.92 * k, c[1] * 0.91 * k, c[2] * 0.9 * k]
        } else {
            [c[0] * k, c[1] * k, c[2] * k]
        }
    });
    let mut rng = t.rng(0x3DF1);
    dabs(
        &mut tile,
        t,
        &mut rng,
        80,
        (1.5, 5.0),
        &[[150, 132, 104], [182, 164, 132], [214, 200, 172]],
        (0.2, 0.4),
        &anywhere,
    );
    tile
}
