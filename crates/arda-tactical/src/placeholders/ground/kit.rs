//! Shared brushwork for the ground painters: multi-scale bases, grass tufts,
//! dabs, pebbles and flowers. Sizes are given for the 256-pixel reference
//! tile and scaled by [`Tex::k`].
// Art generation casts bounded pixel coordinates.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use super::Tex;
use crate::placeholders::brush::Tile;
use crate::placeholders::material::{drift, Rng};
use crate::placeholders::paint::{mix, tone, Rgb};

/// A three-tone palette: dark, mid, light.
pub type Palette = [Rgb; 3];

/// Mixes along a three-tone palette, `t` in `[0, 1]`.
#[must_use]
pub fn ramp(p: &Palette, t: f32) -> Rgb {
    if t < 0.5 {
        mix(p[0], p[1], t * 2.0)
    } else {
        mix(p[1], p[2], (t - 0.5) * 2.0)
    }
}

/// Stretches centred noise so its narrow spread covers `[0, 1]`.
#[must_use]
pub fn spread(n: f32, gain: f32) -> f32 {
    (0.5 + (n - 0.5) * gain).clamp(0.0, 1.0)
}

/// A multi-scale painterly base: broad patches, mid mottling, hue drift and
/// fine grain, all periodic over the tile, then overlaid with hundreds of
/// translucent brush dabs whose tone follows the local patch value.
#[must_use]
pub fn base(t: &Tex, p: &Palette, broad_cells: i64) -> Tile {
    base_with(t, p, broad_cells, 0.2, (0.22, 0.45))
}

/// [`base`] with the dabs' tonal spread and opacity range given, for
/// smooth materials (snow, bare rock) that need calmer brushwork.
#[must_use]
pub fn base_with(
    t: &Tex,
    p: &Palette,
    broad_cells: i64,
    spread_t: f32,
    opacity: (f32, f32),
) -> Tile {
    let broad = |x: f32, y: f32| spread(t.noise(1, x, y, broad_cells, 3), 1.15);
    let mut tile = Tile::new(t.size as u32, |x, y| {
        let mid = t.noise(2, x, y, broad_cells * 4, 2);
        let fine = t.noise(5, x, y, 32, 2);
        let hue = t.noise(3, x, y, broad_cells * 2, 2);
        let grain = t.speckle(4, x, y);
        let c = drift(ramp(p, broad(x, y)), spread(hue, 1.6));
        tone(c, 0.82 + 0.2 * mid + 0.16 * fine + 0.06 * grain)
    });
    let mut rng = t.rng(0xBA5E);
    for _ in 0..t.n(900) {
        let c = spot(t, &mut rng);
        let v = (broad(c.0, c.1) + rng.range(-spread_t, spread_t)).clamp(0.0, 1.0);
        let col = tone(drift(ramp(p, v), rng.f()), rng.range(0.9, 1.08));
        let r = rng.range(2.5, 8.0) * t.k;
        tile.dab(c, r, col, rng.range(opacity.0, opacity.1), rng.next_u64());
    }
    tile
}

/// Wandering cracks: dark polylines with a lit lip on their north-west
/// side, occasionally branching.
#[allow(clippy::too_many_arguments)]
pub fn cracks(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    segs: u32,
    step: f32,
    width: f32,
    dark: Rgb,
) {
    for _ in 0..t.n(n) {
        let mut p = spot(t, rng);
        let mut dir = rng.dir();
        let w0 = width * t.k * rng.range(0.7, 1.2);
        for i in 0..segs {
            let turn = rng.dir();
            dir = norm((dir.0 + 0.45 * turn.0, dir.1 + 0.45 * turn.1));
            let l = step * t.k * rng.range(0.6, 1.3);
            let q = (p.0 + dir.0 * l, p.1 + dir.1 * l);
            let w = w0 * (1.0 - i as f32 / (segs as f32 + 1.0));
            let off = 1.0 * t.k;
            tile.stroke(
                (p.0 - off, p.1 - off),
                (q.0 - off, q.1 - off),
                w * 0.8,
                w * 0.6,
                tone(dark, 2.6),
                0.35,
            );
            tile.stroke(p, q, w, w * 0.8, dark, 0.85);
            if rng.f() < 0.18 {
                let b = rng.dir();
                let e = (p.0 + b.0 * l * 0.8, p.1 + b.1 * l * 0.8);
                tile.stroke(p, e, w * 0.7, 0.2, dark, 0.7);
            }
            p = q;
        }
    }
}

fn norm((x, y): (f32, f32)) -> (f32, f32) {
    let l = (x * x + y * y).sqrt().max(1e-6);
    (x / l, y / l)
}

/// A random point on the tile.
pub fn spot(t: &Tex, rng: &mut Rng) -> (f32, f32) {
    (rng.range(0.0, t.size), rng.range(0.0, t.size))
}

/// A random point where `accept` holds (up to a few tries), else any point.
pub fn spot_where(t: &Tex, rng: &mut Rng, accept: &impl Fn(f32, f32) -> bool) -> (f32, f32) {
    let mut p = spot(t, rng);
    for _ in 0..6 {
        if accept(p.0, p.1) {
            break;
        }
        p = spot(t, rng);
    }
    p
}

/// A colour from the palette with a little jitter.
pub fn shade_of(p: &Palette, rng: &mut Rng, lo: f32, hi: f32) -> Rgb {
    let c = ramp(p, rng.range(lo, hi));
    tone(drift(c, rng.f()), rng.range(0.9, 1.1))
}

/// Grass tufts: `n` clumps of `blades` tapered strokes radiating from a
/// centre. `len` and `width` are in reference pixels.
#[allow(clippy::too_many_arguments)]
pub fn tufts(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    blades: (u32, u32),
    len: (f32, f32),
    width: f32,
    p: &Palette,
    tone_range: (f32, f32),
    accept: &impl Fn(f32, f32) -> bool,
) {
    for _ in 0..t.n(n) {
        let c = spot_where(t, rng, accept);
        let (lean_x, lean_y) = rng.dir();
        let k = blades.0 + (rng.next_u64() % u64::from(blades.1 - blades.0 + 1)) as u32;
        for _ in 0..k {
            let (dx, dy) = rng.dir();
            let (dx, dy) = (dx + 0.5 * lean_x, dy + 0.5 * lean_y);
            let l = rng.range(len.0, len.1) * t.k;
            let a = (
                c.0 + rng.range(-1.5, 1.5) * t.k,
                c.1 + rng.range(-1.5, 1.5) * t.k,
            );
            let b = (a.0 + dx * l, a.1 + dy * l);
            let col = shade_of(p, rng, tone_range.0, tone_range.1);
            tile.stroke(a, b, width * t.k, 0.25 * t.k, col, rng.range(0.75, 1.0));
        }
    }
}

/// Loose strokes in random directions.
#[allow(clippy::too_many_arguments)]
pub fn strokes(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    len: (f32, f32),
    width: (f32, f32),
    p: &Palette,
    opacity: (f32, f32),
) {
    for _ in 0..t.n(n) {
        let a = spot(t, rng);
        let (dx, dy) = rng.dir();
        let l = rng.range(len.0, len.1) * t.k;
        let w = rng.range(width.0, width.1) * t.k;
        let col = shade_of(p, rng, 0.0, 1.0);
        let b = (a.0 + dx * l, a.1 + dy * l);
        tile.stroke(a, b, w, w * 0.6, col, rng.range(opacity.0, opacity.1));
    }
}

/// Soft irregular dabs of colour.
#[allow(clippy::too_many_arguments)]
pub fn dabs(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    r: (f32, f32),
    p: &Palette,
    opacity: (f32, f32),
    accept: &impl Fn(f32, f32) -> bool,
) {
    for _ in 0..t.n(n) {
        let c = spot_where(t, rng, accept);
        let col = shade_of(p, rng, 0.0, 1.0);
        let rr = rng.range(r.0, r.1) * t.k;
        let o = rng.range(opacity.0, opacity.1);
        tile.dab(c, rr, col, o, rng.next_u64());
    }
}

/// Small shaded stones.
#[allow(clippy::too_many_arguments)]
pub fn pebbles(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    r: (f32, f32),
    p: &Palette,
    accept: &impl Fn(f32, f32) -> bool,
) {
    for _ in 0..t.n(n) {
        let c = spot_where(t, rng, accept);
        let rx = rng.range(r.0, r.1) * t.k;
        let ry = rx * rng.range(0.55, 0.95);
        let col = shade_of(p, rng, 0.0, 1.0);
        tile.pebble(c, rx, ry, rng.dir(), col, 1.0);
    }
}

/// Angular stones of radius `r` (reference pixels).
#[allow(clippy::too_many_arguments)]
pub fn stones(
    tile: &mut Tile,
    t: &Tex,
    rng: &mut Rng,
    n: u32,
    r: (f32, f32),
    p: &Palette,
    accept: &impl Fn(f32, f32) -> bool,
) {
    for _ in 0..t.n(n) {
        let c = spot_where(t, rng, accept);
        let rr = rng.range(r.0, r.1) * t.k;
        let col = shade_of(p, rng, 0.0, 1.0);
        let sides = 5 + (rng.next_u64() % 3) as u32;
        tile.stone(c, rr, sides, col, rng.next_u64());
    }
}

/// Tiny flower heads: a coloured dot with a pale centre.
pub fn flowers(tile: &mut Tile, t: &Tex, rng: &mut Rng, n: u32, colours: &[Rgb], r: f32) {
    for _ in 0..t.n(n) {
        let c = spot(t, rng);
        let col = rng.pick(colours).unwrap_or([230, 230, 210]);
        let rr = r * t.k * rng.range(0.7, 1.3);
        tile.dab(c, rr, col, 1.0, rng.next_u64());
        tile.dab(c, rr * 0.35, tone(col, 1.25), 0.9, rng.next_u64());
    }
}

/// Always true.
#[must_use]
pub fn anywhere(_: f32, _: f32) -> bool {
    true
}
