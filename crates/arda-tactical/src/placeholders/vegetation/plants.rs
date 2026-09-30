//! Low plants: bushes, reeds, cattails, ferns, heather, flower patches,
//! tall grass, lily pads and mushroom rings.
// Art generation casts bounded pixel coordinates.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::noise::fbm;
use crate::placeholders::material::{drift, heading, leafy, painted, solid, Rng};
use crate::placeholders::paint::{blob, circle, taper, tone, Rgb};
use crate::placeholders::relief::{bumpy, dome, level, Bounds, Relief, INK, NO_INK, SOFT_INK};

/// A leafy lump.
fn lump(
    r: &mut Relief,
    seed: u64,
    c: (f32, f32),
    rad: f32,
    leaf: Rgb,
    z0: f32,
    ink: crate::placeholders::relief::Ink,
) {
    let bump = move |x: f32, y: f32| fbm(seed ^ 0xB1, x / 2.6, y / 2.6, 2, None);
    r.part(
        Bounds::around(c.0, c.1, rad * 1.25),
        blob(seed, c.0, c.1, rad, 0.2),
        leafy(seed, leaf, (rad * 0.4).max(2.5)),
        bumpy(dome(z0, rad * 0.6, rad), 3.0, bump),
        ink,
    );
}

/// A blade or stalk from `a` to `b`.
fn blade(r: &mut Relief, a: (f32, f32), b: (f32, f32), w: f32, col: Rgb, z0: f32) {
    r.part(
        Bounds::span(a.0, a.1, b.0, b.1, w),
        taper(a, b, w.max(1.1), 1.0),
        solid(col),
        dome(z0, w, w),
        SOFT_INK,
    );
}

/// A round bush of clustered leaves, optionally in flower.
pub fn bush(r: &mut Relief, seed: u64, leaf: Rgb, flowers: bool) {
    let s = r.width as f32;
    let mut rng = Rng::new(seed);
    lump(
        r,
        seed,
        (s * 0.5, s * 0.52),
        s * 0.34,
        tone(leaf, 0.6),
        0.0,
        INK,
    );
    for i in 0..9 {
        let (dx, dy) = heading(i as f32 / 9.0 + rng.range(-0.05, 0.05));
        let p = (s * 0.5 + dx * s * 0.2, s * 0.52 + dy * s * 0.2);
        let col = tone(drift(leaf, rng.f()), 0.85 - 0.06 * (dx + dy));
        lump(
            r,
            rng.next_u64(),
            p,
            s * rng.range(0.13, 0.17),
            col,
            6.0,
            SOFT_INK,
        );
    }
    for _ in 0..4 {
        let p = (s * rng.range(0.4, 0.56), s * rng.range(0.4, 0.56));
        lump(
            r,
            rng.next_u64(),
            p,
            s * rng.range(0.1, 0.14),
            tone(leaf, 1.1),
            12.0,
            SOFT_INK,
        );
    }
    if flowers {
        for _ in 0..14 {
            let (dx, dy) = rng.dir();
            let off = s * rng.range(0.02, 0.3);
            let p = (s * 0.5 + dx * off, s * 0.52 + dy * off);
            r.part(
                Bounds::around(p.0, p.1, 4.0),
                circle(p.0, p.1, 2.3),
                solid([238, 226, 170]),
                dome(18.0, 1.5, 2.3),
                SOFT_INK,
            );
        }
    }
}

/// Reeds: a clump of stalks with seed heads.
pub fn reeds(r: &mut Relief, seed: u64, heads: u32) {
    let s = r.width as f32;
    let c = (s / 2.0, s / 2.0);
    let mut rng = Rng::new(seed);
    for i in 0..46u32 {
        let (dx, dy) = rng.dir();
        let a = (
            c.0 + rng.range(-0.12, 0.12) * s,
            c.1 + rng.range(-0.12, 0.12) * s,
        );
        let l = s * rng.range(0.22, 0.42);
        let b = (a.0 + dx * l, a.1 + dy * l);
        let col = tone(drift([116, 136, 62], rng.f()), rng.range(0.75, 1.2));
        blade(r, a, b, 1.7, col, i as f32 * 0.3);
        if i % heads == 0 {
            let (h0, h1) = (
                (a.0 + dx * l * 0.6, a.1 + dy * l * 0.6),
                (a.0 + dx * l * 0.85, a.1 + dy * l * 0.85),
            );
            r.part(
                Bounds::span(h0.0, h0.1, h1.0, h1.1, 4.0),
                taper(h0, h1, 2.8, 2.4),
                painted(seed, [112, 70, 38], 3.0),
                dome(16.0, 2.5, 2.8),
                INK,
            );
        }
    }
}

/// A fern: fronds with alternating leaflets radiating from a crown.
pub fn fern(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let c = (s / 2.0, s / 2.0);
    let mut rng = Rng::new(seed);
    let n = 8;
    for i in 0..n {
        let (dx, dy) = heading(i as f32 / n as f32 + rng.range(-0.04, 0.04));
        let l = s * rng.range(0.34, 0.44);
        let col = tone(
            drift([70, 110, 44], rng.f()),
            rng.range(0.85, 1.1) * (1.0 - 0.06 * (dx + dy)),
        );
        let tip = (c.0 + dx * l, c.1 + dy * l);
        blade(r, c, tip, 1.4, tone(col, 0.8), 4.0);
        let leaflets = 9;
        for j in 1..leaflets {
            let t = j as f32 / leaflets as f32;
            let at = (c.0 + dx * l * t, c.1 + dy * l * t);
            let len = s * 0.1 * (1.0 - t * 0.8);
            for side in [-1.0f32, 1.0] {
                let (px, py) = (-dy * side, dx * side);
                let e = (at.0 + (px + dx * 0.5) * len, at.1 + (py + dy * 0.5) * len);
                blade(r, at, e, 2.4 * (1.0 - t * 0.5), col, 5.0 + t * 2.0);
            }
        }
    }
}

/// Heather: a low mound of tiny pink-purple flower heads.
pub fn heather(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let mut rng = Rng::new(seed);
    lump(
        r,
        seed,
        (s * 0.5, s * 0.5),
        s * 0.34,
        [58, 70, 40],
        0.0,
        INK,
    );
    for _ in 0..90 {
        let (dx, dy) = rng.dir();
        let off = s * 0.3 * rng.f().sqrt();
        let p = (s * 0.5 + dx * off, s * 0.5 + dy * off);
        let col = tone(drift([160, 96, 142], rng.f()), rng.range(0.75, 1.2));
        r.part(
            Bounds::around(p.0, p.1, 4.0),
            blob(rng.next_u64(), p.0, p.1, 2.6, 0.3),
            solid(col),
            dome(6.0, 2.0, 2.6),
            SOFT_INK,
        );
    }
}

/// A flower patch: low leaves thick with bright blooms.
pub fn flower_patch(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let mut rng = Rng::new(seed);
    for _ in 0..7 {
        let p = (s * rng.range(0.3, 0.7), s * rng.range(0.3, 0.7));
        lump(
            r,
            rng.next_u64(),
            p,
            s * rng.range(0.1, 0.16),
            [72, 112, 48],
            0.0,
            SOFT_INK,
        );
    }
    let blooms = [
        [236, 234, 222],
        [244, 206, 74],
        [206, 70, 70],
        [176, 120, 196],
    ];
    for _ in 0..40 {
        let (dx, dy) = rng.dir();
        let off = s * 0.3 * rng.f().sqrt();
        let p = (s * 0.5 + dx * off, s * 0.5 + dy * off);
        let col = rng.pick(&blooms).unwrap_or(blooms[0]);
        r.part(
            Bounds::around(p.0, p.1, 5.0),
            blob(rng.next_u64(), p.0, p.1, 3.0, 0.25),
            solid(col),
            dome(8.0, 2.0, 3.0),
            SOFT_INK,
        );
        r.part(
            Bounds::around(p.0, p.1, 2.0),
            circle(p.0, p.1, 1.0),
            solid([230, 190, 60]),
            level(10.0),
            NO_INK,
        );
    }
}

/// Tall grass: dense clumps of long blades with a few seed heads.
pub fn tall_grass(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let mut rng = Rng::new(seed);
    for k in 0..5 {
        let c = (s * rng.range(0.32, 0.68), s * rng.range(0.32, 0.68));
        for i in 0..16 {
            let (dx, dy) = rng.dir();
            let l = s * rng.range(0.16, 0.3);
            let b = (c.0 + dx * l, c.1 + dy * l);
            let col = tone(drift([140, 150, 70], rng.f()), rng.range(0.7, 1.15));
            blade(r, c, b, 1.6, col, (k * 16 + i) as f32 * 0.2);
        }
    }
}

/// Cattails: reeds with many plump brown heads.
pub fn cattail(r: &mut Relief, seed: u64) {
    reeds(r, seed, 2);
}

/// Lily pads: notched round leaves floating flat, with a bloom or two.
pub fn lily_pads(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let mut rng = Rng::new(seed);
    for _ in 0..6 {
        let c = (s * rng.range(0.22, 0.78), s * rng.range(0.22, 0.78));
        let rad = s * rng.range(0.08, 0.13);
        let (nx, ny) = rng.dir();
        let pad = circle(c.0, c.1, rad);
        // The notch: a thin wedge cut from the centre outwards.
        let notch = move |x: f32, y: f32| {
            let (dx, dy) = (x - c.0, y - c.1);
            let along = dx * nx + dy * ny;
            let across = (-dx * ny + dy * nx).abs();
            if along > 0.0 {
                across - along * 0.3
            } else {
                f32::MAX
            }
        };
        let col = tone(drift([78, 120, 52], rng.f()), rng.range(0.85, 1.1));
        r.part(
            Bounds::around(c.0, c.1, rad),
            move |x, y| pad(x, y).max(-notch(x, y)),
            leafy(rng.next_u64(), col, 4.0),
            dome(0.0, 2.5, rad),
            INK,
        );
    }
    for _ in 0..2 {
        let c = (s * rng.range(0.3, 0.7), s * rng.range(0.3, 0.7));
        for i in 0..6 {
            let (dx, dy) = heading(i as f32 / 6.0);
            let p = (c.0 + dx * 3.5, c.1 + dy * 3.5);
            r.part(
                Bounds::around(p.0, p.1, 5.0),
                circle(p.0, p.1, 3.2),
                solid([240, 216, 226]),
                dome(4.0, 2.0, 3.2),
                SOFT_INK,
            );
        }
        r.part(
            Bounds::around(c.0, c.1, 3.0),
            circle(c.0, c.1, 2.0),
            solid([236, 200, 70]),
            dome(6.0, 1.0, 2.0),
            NO_INK,
        );
    }
}

/// A fairy ring of small capped mushrooms.
pub fn mushroom_ring(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let c = (s / 2.0, s / 2.0);
    let mut rng = Rng::new(seed);
    let n = 13;
    for i in 0..n {
        let (dx, dy) = heading(i as f32 / n as f32 + rng.range(-0.02, 0.02));
        let off = s * rng.range(0.28, 0.34);
        let p = (c.0 + dx * off, c.1 + dy * off);
        let rad = s * rng.range(0.035, 0.06);
        let red = i % 3 == 0;
        let cap = if red {
            [178, 52, 40]
        } else {
            tone([176, 138, 92], rng.range(0.85, 1.1))
        };
        r.part(
            Bounds::around(p.0, p.1, rad),
            circle(p.0, p.1, rad),
            painted(rng.next_u64(), cap, 3.0),
            dome(0.0, rad * 0.7, rad),
            INK,
        );
        if red {
            for _ in 0..3 {
                let q = (
                    p.0 + rng.range(-0.5, 0.5) * rad,
                    p.1 + rng.range(-0.5, 0.5) * rad,
                );
                r.part(
                    Bounds::around(q.0, q.1, 2.0),
                    circle(q.0, q.1, 1.1),
                    solid([240, 236, 224]),
                    level(rad * 0.7),
                    NO_INK,
                );
            }
        }
    }
}
