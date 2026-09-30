//! Laid surfaces: cobbles, ashlar stone floors, flagstone yards, plank
//! floors and rugs. Their layout comes from the shared layout seed; variants
//! differ only in tint, wear, dirt and moss.

use super::kit::{anywhere, base, dabs, strokes, Palette};
use super::Tex;
use crate::noise::{hash2, mix as mix64, unit, value};
use crate::placeholders::brush::Tile;
use crate::placeholders::paint::{mix, tone, Rgb};

const GROUT: Palette = [[58, 50, 38], [80, 70, 54], [102, 92, 72]];
const MOSS: Palette = [[62, 76, 40], [84, 98, 52], [108, 118, 68]];

fn f3(c: Rgb) -> [f32; 3] {
    [f32::from(c[0]), f32::from(c[1]), f32::from(c[2])]
}

/// Dome-like shading of a stone: lit towards the top-left near its edge.
fn stone_lit(x: f32, y: f32, at: (f32, f32), edge: f32, cell: f32) -> f32 {
    let (dx, dy) = ((x - at.0) / cell, (y - at.1) / cell);
    let h = (edge / (cell * 0.3)).clamp(0.0, 1.0);
    1.0 + 0.55 * (-(dx + dy)) * (1.0 - h) + 0.06 * h
}

/// Cobbles: rounded stones set in dirt with moss in some joints.
pub fn cobbles(t: &Tex) -> Tile {
    let n = 11 * i64::from(t.span) / 2;
    let cell = t.size / n as f32;
    let mut tile = base(t, &GROUT, 6);
    let mossy = t.seed.is_multiple_of(3);
    tile.map(|x, y, c| {
        let (f1, f2, id, at) = t.voronoi(x, y, n, 0.75);
        let edge = f2 - f1;
        let _ = f1;
        if edge < 3.0 * t.k {
            if mossy && t.noise(5, x, y, 6, 2) > 0.55 {
                return f3(mix(MOSS[1], MOSS[2], t.speckle(6, x, y)));
            }
            return c;
        }
        let tint = unit(mix64(id ^ t.seed));
        let hue = unit(mix64(id ^ t.seed ^ 0x77));
        let stone = mix([108, 104, 98], [178, 170, 152], tint);
        let stone = mix(stone, [150, 128, 100], hue * 0.35);
        let k = stone_lit(x, y, at, edge, cell) * (0.9 + 0.18 * t.noise(7, x, y, 40, 2));
        let s = tone(stone, k);
        let rim = if edge < 4.5 * t.k { 0.6 } else { 1.0 };
        let s = tone(s, rim);
        f3(s)
    });
    let mut rng = t.rng(0xC0B1);
    dabs(
        &mut tile,
        t,
        &mut rng,
        20,
        (5.0, 12.0),
        &GROUT,
        (0.12, 0.25),
        &anywhere,
    );
    tile
}

/// A slab-edge shade: lit top-left edges, dark bottom-right edges, a grout
/// line, and the slab face with its own tint and mottling.
fn slab(face: [f32; 3], grout: [f32; 3], ex: (f32, f32), ey: (f32, f32), k: f32) -> [f32; 3] {
    // ex = (distance to west edge, to east edge); ey likewise north/south.
    let e = ex.0.min(ex.1).min(ey.0).min(ey.1);
    if e < 1.6 * k {
        return grout;
    }
    let bev = 3.2 * k;
    let lit = if ex.0 < bev || ey.0 < bev {
        1.14
    } else if ex.1 < bev || ey.1 < bev {
        0.8
    } else {
        1.0
    };
    [face[0] * lit, face[1] * lit, face[2] * lit]
}

/// Ashlar stone floor: courses of cut slabs, offset per course. Courses
/// are shifted half a course so the tile's wrap seam falls mid-slab.
pub fn stone_floor(t: &Tex) -> Tile {
    let rows = 4.0;
    let rh = t.size / rows;
    let pattern = [[0.0, 0.25, 0.625], [0.0, 0.375, 0.75], [0.0, 0.5, 0.75]];
    let p = [[150, 142, 126], [178, 170, 152], [204, 196, 176]];
    Tile::new(t.size as u32, |x, y| {
        let yy = (y + rh / 2.0).rem_euclid(t.size);
        let row = (yy / rh).floor();
        let h = hash2(t.structure ^ 0xF1A6, row as i64, 0);
        let cuts = pattern[(h % 3) as usize];
        let shift = unit(mix64(h));
        let u = (x / t.size + shift).rem_euclid(1.0);
        let mut slab_i = 0;
        for (i, cut) in cuts.iter().enumerate() {
            if u >= *cut {
                slab_i = i;
            }
        }
        let start = cuts[slab_i];
        let end = cuts.get(slab_i + 1).copied().unwrap_or(1.0);
        let (west, east) = ((u - start) * t.size, (end - u) * t.size);
        let (north, south) = (yy - row * rh, (row + 1.0) * rh - yy);
        let layout = unit(hash2(t.structure ^ 0xF1A8, row as i64, slab_i as i64));
        let tint = unit(hash2(t.seed ^ 0xF1A7, row as i64, slab_i as i64));
        let broad = super::kit::spread(t.layout_noise(1, x, y, 4, 3), 1.6);
        let c = super::kit::ramp(&p, (0.7 * broad + 0.3 * layout).clamp(0.0, 1.0));
        let k = (0.92 + 0.1 * tint)
            * (0.9 + 0.14 * t.layout_noise(2, x, y, 16, 2) + 0.06 * t.noise(3, x, y, 24, 2));
        let stain = t.noise(9, x, y, 5, 3);
        let mut face = [
            f32::from(c[0]) * k,
            f32::from(c[1]) * k,
            f32::from(c[2]) * k,
        ];
        if stain > 0.7 {
            let s = ((stain - 0.7) * 1.2).min(0.12);
            face = [
                face[0] * (1.0 - s),
                face[1] * (1.0 - s * 1.05),
                face[2] * (1.0 - s * 1.2),
            ];
        }
        let out = slab(
            face,
            [104.0, 96.0, 84.0],
            (west, east),
            (north, south),
            t.k.max(0.7),
        );
        [
            crate::placeholders::paint::byte(out[0]),
            crate::placeholders::paint::byte(out[1]),
            crate::placeholders::paint::byte(out[2]),
        ]
    })
}

/// Flagstone yard: large irregular slabs with grass and moss in the joints.
pub fn flagstone(t: &Tex) -> Tile {
    let n = 5 * i64::from(t.span) / 2;
    let cell = t.size / n as f32;
    let mut tile = base(t, &MOSS, 4);
    tile.map(|x, y, c| {
        let (x, y) = (x + cell * 0.5, y + cell * 0.5);
        let (f1, f2, id, at) = t.voronoi(x, y, n, 0.6);
        let edge = f2 - f1;
        let _ = f1;
        if edge < 3.4 * t.k {
            return if t.noise(3, x, y, 8, 2) > 0.5 {
                c
            } else {
                f3(GROUT[1])
            };
        }
        let tint = unit(mix64(id ^ t.seed));
        let face = mix([150, 140, 122], [196, 186, 164], tint);
        let (dx, dy) = ((x - at.0) / cell, (y - at.1) / cell);
        let rim = if edge < 6.0 * t.k {
            1.0 - 0.35 * (dx + dy)
        } else {
            1.0
        };
        let k = rim * (0.88 + 0.2 * t.noise(4, x, y, 20, 3));
        f3(tone(face, k))
    });
    tile
}

/// Plank floor: boards running east–west with grain, joints and nails.
pub fn planks(t: &Tex) -> Tile {
    let rows = 8.0;
    let rh = t.size / rows;
    let mut tile = Tile::new(t.size as u32, |x, y| {
        // Rows are shifted half a board so the wrap seam falls mid-board.
        let y = (y + rh / 2.0).rem_euclid(t.size);
        let row = (y / rh).floor();
        let h = hash2(t.structure ^ 0x9A4C, row as i64, 0);
        let joint = unit(h) * t.size;
        let rel = (x - joint).rem_euclid(t.size);
        let half = t.size / 2.0;
        let dj = rel.min(t.size - rel).min((rel - half).abs());
        let seg = i64::from(rel > half);
        let tint = unit(hash2(t.seed ^ 0x9A4D, row as i64, seg));
        let shared = unit(hash2(t.structure ^ 0x9A4E, row as i64, seg));
        let board = mix([112, 78, 48], [160, 118, 76], 0.7 * shared + 0.3 * tint);
        let sd = mix64(t.structure ^ (row as u64) << 3 ^ seg as u64);
        // Grain: periodic along x (4 and 16 cycles across the tile).
        let g = t.layout_noise(1 ^ sd, x, y * 12.0, 4, 2);
        let fine = value(
            sd,
            (x / t.size * 64.0).rem_euclid(64.0),
            y / t.size * 128.0,
            Some(64),
        );
        let c = tone(board, 0.78 + 0.28 * g + 0.12 * fine);
        let dy = (y - row * rh).min((row + 1.0) * rh - y);
        let top = y - row * rh;
        if dy < 1.3 * t.k || dj < 1.0 * t.k {
            return tone(board, 0.38);
        }
        let lit = if top < 2.6 * t.k {
            1.12
        } else if (row + 1.0) * rh - y < 2.6 * t.k {
            0.84
        } else {
            1.0
        };
        let nail = (dj - 4.0 * t.k).abs() < 1.2 * t.k && (top - rh * 0.5).abs() < 1.2 * t.k;
        if nail {
            [54, 50, 48]
        } else {
            tone(c, lit)
        }
    });
    let mut rng = t.rng(0x9A4E);
    strokes(
        &mut tile,
        t,
        &mut rng,
        30,
        (6.0, 18.0),
        (0.5, 0.9),
        &[[70, 50, 32], [90, 66, 42], [110, 84, 56]],
        (0.2, 0.4),
    );
    tile
}

/// Rug: a woven diamond pattern in madder, indigo and ochre.
pub fn rug(t: &Tex) -> Tile {
    let reps = 2.0;
    let p = t.size / reps;
    let palettes: [[Rgb; 4]; 2] = [
        [[140, 40, 36], [196, 150, 70], [40, 50, 84], [226, 208, 170]],
        [
            [46, 64, 104],
            [184, 120, 58],
            [128, 36, 40],
            [220, 204, 168],
        ],
    ];
    let pal = palettes[(t.structure % 2) as usize];
    Tile::new(t.size as u32, |x, y| {
        let y = y + p / 16.0 + 0.5;
        let (u, v) = ((x / p).rem_euclid(1.0) - 0.5, (y / p).rem_euclid(1.0) - 0.5);
        let dia = u.abs() + v.abs();
        let ring = (dia * 8.0).floor() as i64;
        let band = ((y / p * 8.0).floor() as i64).rem_euclid(8);
        let col = match ring {
            0 => pal[1],
            1 => pal[3],
            2 | 3 => pal[0],
            4 => pal[2],
            _ if band == 0 => pal[2],
            _ => pal[0],
        };
        let weave = if (x as i64 + (y as i64 / 2)) % 2 == 0 {
            1.0
        } else {
            0.93
        };
        let wear = 0.86 + 0.14 * t.noise(3, x, y, 6, 3) + 0.08 * t.speckle(4, x, y);
        tone(col, weave * wear)
    })
}
