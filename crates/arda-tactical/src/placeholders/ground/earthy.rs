//! Bare and covered earth: dirt, mud, sand, gravel, packed earth, ploughed
//! farmland, forest floor, leaf litter, snow and ice.

use super::kit::{
    anywhere, base, base_with, dabs, flowers, pebbles, ramp, spot, spread, strokes, tufts, Palette,
};
use super::Tex;
use crate::placeholders::brush::Tile;
use crate::placeholders::paint::{mix, tone};

const DIRT: Palette = [[98, 76, 52], [136, 110, 76], [170, 146, 106]];
const STONES: Palette = [[112, 106, 96], [150, 142, 126], [188, 178, 158]];
const TWIGS: Palette = [[70, 50, 32], [98, 72, 46], [126, 98, 66]];

/// Dirt: warm brown patches, scuffs, pebbles and twigs.
pub fn dirt(t: &Tex) -> Tile {
    let mut tile = base(t, &DIRT, 4);
    let mut rng = t.rng(0xD127);
    dabs(
        &mut tile,
        t,
        &mut rng,
        120,
        (4.0, 12.0),
        &DIRT,
        (0.25, 0.5),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        320,
        (1.5, 4.0),
        &DIRT,
        (0.35, 0.7),
        &anywhere,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        260,
        (3.0, 10.0),
        (0.6, 1.3),
        &DIRT,
        (0.3, 0.6),
    );
    pebbles(&mut tile, t, &mut rng, 110, (1.2, 3.2), &STONES, &anywhere);
    strokes(
        &mut tile,
        t,
        &mut rng,
        10,
        (6.0, 12.0),
        (0.6, 0.9),
        &TWIGS,
        (0.8, 1.0),
    );
    tile
}

/// Mud: dark wet earth with glossy highlights, ruts and puddles.
pub fn mud(t: &Tex) -> Tile {
    const P: Palette = [[58, 46, 32], [84, 66, 46], [112, 92, 64]];
    let mut tile = base(t, &P, 4);
    let mut rng = t.rng(0x3D00);
    dabs(
        &mut tile,
        t,
        &mut rng,
        90,
        (5.0, 14.0),
        &P,
        (0.3, 0.55),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        260,
        (1.5, 4.0),
        &P,
        (0.35, 0.65),
        &anywhere,
    );
    let wet = |x: f32, y: f32| spread(t.noise(5, x, y, 4, 3), 2.2);
    tile.map(|x, y, c| {
        let w = wet(x, y);
        if w > 0.7 {
            let k = ((w - 0.7) * 5.0).min(0.8);
            let pool = [52.0, 50.0, 42.0];
            [
                c[0] + (pool[0] - c[0]) * k,
                c[1] + (pool[1] - c[1]) * k,
                c[2] + (pool[2] - c[2]) * k,
            ]
        } else {
            c
        }
    });
    for _ in 0..t.n(60) {
        let a = spot(t, &mut rng);
        let (dx, dy) = rng.dir();
        let l = rng.range(4.0, 10.0) * t.k;
        tile.stroke(
            a,
            (a.0 + dx * l, a.1 + dy * l),
            0.9 * t.k,
            0.3 * t.k,
            [150, 140, 120],
            rng.range(0.25, 0.5),
        );
    }
    pebbles(&mut tile, t, &mut rng, 20, (1.2, 2.6), &STONES, &anywhere);
    tile
}

/// Sand: pale warm grains in wind ripples, with shell bits and pebbles.
pub fn sand(t: &Tex) -> Tile {
    const P: Palette = [[178, 156, 112], [206, 186, 140], [228, 212, 170]];
    let mut tile = base_with(t, &P, 5, 0.18, (0.2, 0.4));
    let mut rng = t.rng(0x5A2D);
    tile.map(|x, y, c| {
        let r = t.noise(6, x + 6.0 * t.noise(7, x, y, 3, 2), y * 4.0, 12, 2);
        let k = 0.95 + 0.1 * r;
        [c[0] * k, c[1] * k, c[2] * k]
    });
    dabs(
        &mut tile,
        t,
        &mut rng,
        60,
        (4.0, 10.0),
        &P,
        (0.2, 0.4),
        &anywhere,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        300,
        (2.0, 6.0),
        (0.5, 0.9),
        &P,
        (0.3, 0.6),
    );
    pebbles(&mut tile, t, &mut rng, 24, (1.0, 2.4), &STONES, &anywhere);
    dabs(
        &mut tile,
        t,
        &mut rng,
        24,
        (0.8, 1.4),
        &[[236, 228, 210], [240, 234, 220], [226, 214, 196]],
        (0.9, 1.0),
        &anywhere,
    );
    tile
}

/// Gravel: a bed densely packed with small shaded stones.
pub fn gravel(t: &Tex) -> Tile {
    const BED: Palette = [[88, 76, 60], [114, 100, 80], [140, 126, 104]];
    let mut tile = base(t, &BED, 5);
    let mut rng = t.rng(0x62A7);
    pebbles(&mut tile, t, &mut rng, 900, (2.0, 4.6), &STONES, &anywhere);
    pebbles(&mut tile, t, &mut rng, 500, (1.2, 2.6), &STONES, &anywhere);
    tile
}

/// Packed earth: a trodden floor, smooth with fine scuffs and straw.
pub fn packed_earth(t: &Tex) -> Tile {
    const P: Palette = [[112, 90, 64], [142, 118, 86], [168, 146, 110]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x9AC4);
    dabs(
        &mut tile,
        t,
        &mut rng,
        70,
        (6.0, 16.0),
        &P,
        (0.15, 0.35),
        &anywhere,
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        240,
        (1.5, 4.0),
        &P,
        (0.3, 0.55),
        &anywhere,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        260,
        (4.0, 14.0),
        (0.5, 1.0),
        &P,
        (0.25, 0.5),
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        30,
        (3.0, 7.0),
        (0.4, 0.7),
        &[[190, 168, 96], [206, 186, 112], [170, 146, 80]],
        (0.6, 0.9),
    );
    pebbles(&mut tile, t, &mut rng, 14, (1.0, 2.0), &STONES, &anywhere);
    tile
}

/// Ploughed farmland: furrows running east–west with clods and sprouts.
/// The furrow layout is shared by every variant.
pub fn farmland(t: &Tex) -> Tile {
    const SOIL: Palette = [[70, 52, 34], [104, 80, 54], [140, 112, 78]];
    let rows = 8.0;
    let period = t.size / rows;
    let wob = |x: f32| 3.0 * t.k * (t.layout_noise(3, x, 0.0, 4, 2) - 0.5) * 2.0;
    let mut tile = base(t, &SOIL, 3);
    tile.map(|x, y, c| {
        let v = ((y + wob(x) + period * 0.5) / period).rem_euclid(1.0);
        // A rounded ridge crest at v = 0.5; the north-facing flank (towards
        // the light) is bright, the south-facing flank and trough dark.
        let ridge = 1.0 - (2.0 * v - 1.0).abs();
        let flank = if v < 0.5 { 0.16 } else { -0.2 };
        let k = 0.62 + 0.42 * ridge + flank * (1.0 - ridge);
        [c[0] * k, c[1] * k, c[2] * k]
    });
    let mut rng = t.rng(0xFA23);
    let crest = |_x: f32, y: f32| {
        let v = ((y + period * 0.5) / period).rem_euclid(1.0);
        (v - 0.5).abs() < 0.2
    };
    pebbles(&mut tile, t, &mut rng, 260, (1.4, 3.2), &SOIL, &crest);
    strokes(
        &mut tile,
        t,
        &mut rng,
        60,
        (3.0, 7.0),
        (0.4, 0.7),
        &[[150, 130, 80], [170, 150, 96], [130, 112, 70]],
        (0.5, 0.8),
    );
    let mut lay = t.layout_rng(0xFA24);
    let sprouts = t.seed.is_multiple_of(2);
    if sprouts {
        for _ in 0..t.n(220) {
            let x = lay.range(0.0, t.size);
            let row = (lay.range(0.0, rows)).floor();
            let y = row * period - wob(x);
            let (dx, dy) = rng.dir();
            let l = rng.range(2.0, 4.0) * t.k;
            let col = mix([80, 110, 44], [130, 150, 66], rng.f());
            tile.stroke(
                (x, y),
                (x + dx * l, y + dy * l),
                1.0 * t.k,
                0.3 * t.k,
                col,
                0.95,
            );
        }
    }
    tile
}

/// Forest floor: dark humus, needles, twigs, small leaves and moss.
pub fn forest_floor(t: &Tex) -> Tile {
    const P: Palette = [[44, 42, 30], [64, 60, 42], [86, 80, 58]];
    const NEEDLES: Palette = [[84, 70, 50], [104, 90, 64], [126, 112, 82]];
    const MOSS: Palette = [[48, 70, 30], [74, 98, 40], [108, 128, 56]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0xF02E);
    let mossy = |x: f32, y: f32| t.noise(8, x, y, 4, 2) > 0.55;
    dabs(
        &mut tile,
        t,
        &mut rng,
        60,
        (4.0, 10.0),
        &MOSS,
        (0.5, 0.8),
        &mossy,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        700,
        (3.0, 7.0),
        (0.35, 0.6),
        &NEEDLES,
        (0.6, 0.95),
    );
    leaves(
        &mut tile,
        t,
        &mut rng,
        50,
        &[[120, 84, 42], [96, 70, 40], [136, 104, 56]],
        3.0,
    );
    strokes(
        &mut tile,
        t,
        &mut rng,
        14,
        (10.0, 22.0),
        (0.7, 1.2),
        &TWIGS,
        (0.9, 1.0),
    );
    tufts(
        &mut tile,
        t,
        &mut rng,
        30,
        (3, 5),
        (3.0, 6.0),
        1.0,
        &MOSS,
        (0.4, 1.0),
        &mossy,
    );
    pebbles(&mut tile, t, &mut rng, 10, (1.4, 3.0), &STONES, &anywhere);
    tile
}

/// Leaf litter: a carpet of fallen autumn leaves.
pub fn leaf_litter(t: &Tex) -> Tile {
    const P: Palette = [[58, 50, 34], [82, 70, 48], [106, 94, 64]];
    let mut tile = base(t, &P, 3);
    let mut rng = t.rng(0x1EAF);
    let autumn = [
        [124, 88, 52],
        [146, 118, 66],
        [100, 74, 46],
        [128, 124, 70],
        [86, 64, 42],
        [156, 132, 80],
    ];
    leaves(&mut tile, t, &mut rng, 520, &autumn, 4.2);
    leaves(&mut tile, t, &mut rng, 200, &autumn, 3.2);
    strokes(
        &mut tile,
        t,
        &mut rng,
        8,
        (8.0, 16.0),
        (0.6, 1.0),
        &TWIGS,
        (0.9, 1.0),
    );
    tile
}

/// Pointed leaves with a darker midrib.
fn leaves(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut crate::placeholders::material::Rng,
    n: u32,
    colours: &[[u8; 3]],
    size: f32,
) {
    for _ in 0..t.n(n) {
        let c = spot(t, rng);
        let (dx, dy) = rng.dir();
        let l = size * t.k * rng.range(0.8, 1.3);
        let w = l * 0.42;
        let col = tone(
            rng.pick(colours).unwrap_or([140, 90, 40]),
            rng.range(0.85, 1.12),
        );
        let a = (c.0 - dx * l, c.1 - dy * l);
        let b = (c.0 + dx * l, c.1 + dy * l);
        tile.stroke(c, a, w, 0.2, col, 1.0);
        tile.stroke(c, b, w, 0.2, col, 1.0);
        tile.stroke(a, b, 0.35 * t.k, 0.2 * t.k, tone(col, 0.62), 0.8);
    }
}

/// Snow: blue-white drifts with soft wind ripples and sparkles.
pub fn snow(t: &Tex) -> Tile {
    const P: Palette = [[200, 212, 230], [228, 236, 246], [248, 250, 255]];
    let mut tile = base_with(t, &P, 3, 0.12, (0.15, 0.3));
    let mut rng = t.rng(0x5E0F);
    tile.map(|x, y, c| {
        // Wind-carved drifts: soft blue troughs, bright crests.
        let r = t.noise(6, x + 10.0 * t.noise(7, x, y, 3, 2), y * 3.0, 8, 2);
        let k = 0.95 + 0.08 * r;
        [c[0] * k, c[1] * k, (c[2] * (k + 0.015)).min(255.0)]
    });
    strokes(
        &mut tile,
        t,
        &mut rng,
        220,
        (4.0, 12.0),
        (0.6, 1.4),
        &[[196, 208, 228], [222, 230, 244], [252, 253, 255]],
        (0.25, 0.5),
    );
    dabs(
        &mut tile,
        t,
        &mut rng,
        40,
        (1.0, 2.2),
        &[[150, 150, 150], [130, 128, 124], [170, 168, 160]],
        (0.5, 0.8),
        &|x, y| t.noise(8, x, y, 4, 2) > 0.6,
    );
    flowers(&mut tile, t, &mut rng, 40, &[[255, 255, 255]], 0.7);
    tile
}

/// Ice: pale cyan with white cracks, frost and trapped bubbles.
pub fn ice(t: &Tex) -> Tile {
    const P: Palette = [[128, 168, 186], [168, 202, 214], [206, 228, 236]];
    let mut tile = base(t, &P, 3);
    tile.map(|x, y, c| {
        let (f1, f2, _, _) = t.voronoi(x, y, 5, 0.8);
        let (g1, g2, _, _) = t.voronoi(x * 1.0, y, 11, 0.9);
        let edge = f2 - f1;
        let fine = g2 - g1;
        let mut c = c;
        if edge < 1.6 * t.k {
            c = [c[0] * 0.2 + 200.0, c[1] * 0.2 + 210.0, c[2] * 0.2 + 214.0];
        } else if edge < 2.6 * t.k {
            c = [c[0] * 0.82, c[1] * 0.86, c[2] * 0.9];
        } else if fine < 0.8 * t.k && f1 > 20.0 * t.k {
            c = [c[0] * 1.06, c[1] * 1.05, c[2] * 1.03];
        }
        c
    });
    let mut rng = t.rng(0x1CE0);
    dabs(
        &mut tile,
        t,
        &mut rng,
        30,
        (5.0, 14.0),
        &[[220, 236, 242], [236, 244, 248], [200, 222, 232]],
        (0.2, 0.45),
        &anywhere,
    );
    flowers(&mut tile, t, &mut rng, 30, &[[214, 232, 240]], 0.9);
    let _ = ramp(&P, 0.5);
    tile
}
