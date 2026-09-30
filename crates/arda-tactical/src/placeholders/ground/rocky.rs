//! Rocky ground: scree, bare rock and cliff faces.
//!
//! Scree is a pile of painted angular stones. Bedrock and cliffs are broad
//! mottled plates whose cracks are broken up by noise and lit on one lip,
//! so they read as weathered stone rather than a crackle glaze.

use super::kit::{anywhere, base, base_with, cracks, dabs, pebbles, spread, stones, Palette};
use super::Tex;
use crate::noise::{mix as mix64, unit};
use crate::placeholders::brush::Tile;

const ROCK: Palette = [[98, 94, 86], [140, 134, 122], [184, 178, 164]];
const LICHEN: Palette = [[120, 128, 78], [150, 150, 96], [176, 170, 120]];

/// Scree: loose angular fragments of mixed sizes over a gritty bed.
pub fn scree(t: &Tex) -> Tile {
    const BED: Palette = [[70, 66, 60], [100, 96, 88], [128, 122, 112]];
    let mut tile = base(t, &BED, 5);
    let mut rng = t.rng(0x5C2E);
    pebbles(&mut tile, t, &mut rng, 500, (1.0, 2.6), &ROCK, &anywhere);
    stones(&mut tile, t, &mut rng, 260, (3.0, 7.0), &ROCK, &anywhere);
    stones(&mut tile, t, &mut rng, 90, (7.0, 13.0), &ROCK, &anywhere);
    stones(&mut tile, t, &mut rng, 140, (2.5, 5.0), &ROCK, &anywhere);
    tile
}

/// Bare rock: broad weathered plates split by wandering cracks, lichen and
/// a few loose stones.
pub fn rock(t: &Tex) -> Tile {
    let mut tile = base_with(t, &ROCK, 3, 0.15, (0.2, 0.4));
    tile.map(|x, y, c| {
        let (_, _, id, _) = t.voronoi(x, y, 3, 0.9);
        let plate = 0.9 + 0.18 * unit(mix64(id ^ t.seed));
        let mottle = 0.9 + 0.2 * t.noise(31, x, y, 12, 3);
        let k = plate * mottle;
        [c[0] * k, c[1] * k, c[2] * k]
    });
    let mut rng = t.rng(0x20CC);
    cracks(&mut tile, t, &mut rng, 10, 9, 9.0, 1.5, [52, 48, 44]);
    cracks(&mut tile, t, &mut rng, 22, 4, 6.0, 1.0, [70, 66, 60]);
    let lichen = |x: f32, y: f32| t.noise(33, x, y, 6, 2) > 0.56;
    dabs(
        &mut tile,
        t,
        &mut rng,
        110,
        (1.5, 4.0),
        &LICHEN,
        (0.55, 0.85),
        &lichen,
    );
    pebbles(&mut tile, t, &mut rng, 90, (1.0, 2.4), &ROCK, &anywhere);
    stones(&mut tile, t, &mut rng, 40, (2.5, 6.0), &ROCK, &anywhere);
    tile
}

/// Cliff face: strata bands, broken fissures, rubble and lichen.
pub fn cliff(t: &Tex) -> Tile {
    const P: Palette = [[78, 74, 68], [118, 112, 102], [164, 158, 144]];
    let mut tile = base(t, &P, 3);
    let bands = 6.0;
    let bh = t.size / bands;
    tile.map(|x, y, c| {
        let wy = y + bh * 0.5 + 10.0 * t.k * (t.layout_noise(41, x, y, 3, 2) - 0.5) * 2.0;
        let band = (wy / bh).rem_euclid(bands);
        let bi = band.floor();
        let v = band - bi;
        let tint = unit(mix64(t.structure ^ bi as u64));
        // Each stratum is lit on its upper (north) lip and darkens below.
        let lip = if v < 0.08 { 1.14 } else { 1.0 - 0.3 * v };
        let grain = spread(t.noise(42, x, y, 12, 2), 1.6);
        let k = (0.8 + 0.34 * tint) * lip * (0.9 + 0.18 * grain);
        [c[0] * k, c[1] * k, c[2] * k]
    });
    let mut lay = t.layout_rng(0xC11F);
    cracks(&mut tile, t, &mut lay, 12, 7, 8.0, 1.8, [40, 36, 32]);
    let mut rng = t.rng(0xC120);
    stones(&mut tile, t, &mut rng, 40, (2.5, 6.0), &P, &anywhere);
    let lichen = |x: f32, y: f32| t.noise(43, x, y, 5, 2) > 0.6;
    dabs(
        &mut tile,
        t,
        &mut rng,
        50,
        (1.2, 3.0),
        &LICHEN,
        (0.5, 0.8),
        &lichen,
    );
    tile
}
