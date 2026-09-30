//! Farm and yard props: hay bales, troughs, millstones, wheelbarrows,
//! ladders and hay carts.

use super::yard::wheel;
use super::{plank_rect, pole, rect, DARK_WOOD, EAST, HAY, PALE_WOOD, SOUTH, WOOD};
use crate::placeholders::material::{solid, stone, straw, wood, Rng};
use crate::placeholders::paint::{blob, circle, mix, rbox};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

/// Hay bale: a rectangular bale of straw bound by two twine bands.
pub fn hay_bale(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let (hw, hh) = (s * 0.36, s * 0.24);
    r.part(
        Bounds::around(c.0, c.1, hw),
        rbox(c.0, c.1, hw, hh, 7.0),
        straw(seed, HAY, EAST),
        bevel(0.0, 10.0, 8.0),
        INK,
    );
    for dx in [-0.16, 0.16] {
        rect(
            r,
            (c.0 + dx * s, c.1),
            (1.6, hh - 1.0),
            0.5,
            solid([150, 110, 60]),
            10.0,
            0.5,
            SOFT_INK,
        );
    }
}

/// Trough (2 × 1): a long plank trough full of water.
pub fn trough(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    plank_rect(r, seed, c, (w * 0.44, s * 0.2), WOOD, EAST, 0.0, 0.0);
    rect(
        r,
        c,
        (w * 0.4, s * 0.14),
        2.0,
        |x, y, _| {
            mix(
                [44, 76, 86],
                [86, 130, 136],
                ((x * 0.3 + y) / 30.0).fract() * 0.5,
            )
        },
        2.0,
        0.0,
        SOFT_INK,
    );
    for x in [w * 0.06, w * 0.94] {
        plank_rect(
            r,
            seed ^ 1,
            (x, c.1),
            (s * 0.03, s * 0.22),
            DARK_WOOD,
            SOUTH,
            0.0,
            6.0,
        );
    }
}

/// Millstone: a great round stone with a square eye and dressed grooves.
pub fn millstone(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let base = stone(seed, [150, 144, 132]);
    r.part(
        Bounds::around(c.0, c.1, s * 0.44),
        circle(c.0, c.1, s * 0.43),
        move |x, y, d| {
            let (dx, dy) = (x - c.0, y - c.1);
            // Grooves: harrow dressing in six sectors (no trigonometry).
            let groove = ((dx * 0.5 + dy * 0.866).abs() < 1.2
                || (dx * 0.5 - dy * 0.866).abs() < 1.2
                || dy.abs() < 1.2)
                && (dx * dx + dy * dy) > (s * 0.1) * (s * 0.1);
            let col = base(x, y, d);
            if groove {
                crate::placeholders::paint::tone(col, 0.7)
            } else {
                col
            }
        },
        bevel(0.0, 10.0, 7.0),
        INK,
    );
    rect(
        r,
        c,
        (s * 0.06, s * 0.06),
        1.0,
        solid([46, 42, 38]),
        6.0,
        0.0,
        SOFT_INK,
    );
}

/// Wheelbarrow: a tray with a wheel to the north and handles to the south.
pub fn wheelbarrow(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    wheel(r, seed, (c.0, s * 0.13), s * 0.1, s * 0.035, 0.0);
    for dx in [-0.16, 0.16] {
        pole(
            r,
            seed,
            (c.0 + dx * 0.6 * s, s * 0.14),
            (c.0 + dx * s, s * 0.92),
            2.2,
            WOOD,
            6.0,
        );
    }
    let tray = rbox(c.0, c.1 - s * 0.02, s * 0.22, s * 0.26, 6.0);
    r.part(
        Bounds::around(c.0, c.1, s * 0.3),
        tray,
        wood(seed ^ 1, PALE_WOOD, SOUTH, s * 0.07),
        bevel(8.0, -4.0, 5.0),
        INK,
    );
    r.part(
        Bounds::around(c.0, c.1, s * 0.2),
        blob(seed ^ 2, c.0, c.1, s * 0.15, 0.15),
        crate::placeholders::material::painted(seed, [110, 88, 60], 4.0),
        dome(6.0, 6.0, s * 0.15),
        SOFT_INK,
    );
}

/// Ladder (1 × 2): two rails and rungs, lying flat.
pub fn ladder(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    for x in [w / 2.0 - s * 0.18, w / 2.0 + s * 0.18] {
        pole(r, seed, (x, s * 0.08), (x, h - s * 0.08), 3.4, WOOD, 4.0);
    }
    let n = 7;
    for i in 0..n {
        let y = s * 0.24 + (h - s * 0.48) * i as f32 / (n - 1) as f32;
        pole(
            r,
            seed ^ i,
            (w / 2.0 - s * 0.18, y),
            (w / 2.0 + s * 0.18, y),
            2.4,
            PALE_WOOD,
            2.0,
        );
    }
}

/// Hay cart (1 × 2): a cart heaped with hay.
pub fn haycart(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    for x in [cx - s * 0.42, cx + s * 0.42] {
        wheel(r, seed, (x, h * 0.62), s * 0.3, s * 0.06, 0.0);
    }
    for x in [cx - s * 0.18, cx + s * 0.18] {
        pole(r, seed, (x, s * 0.06), (x, h * 0.4), s * 0.035, WOOD, 4.0);
    }
    plank_rect(
        r,
        seed,
        (cx, h * 0.6),
        (s * 0.36, s * 0.62),
        WOOD,
        SOUTH,
        s * 0.12,
        6.0,
    );
    let mut rng = Rng::new(seed);
    for i in 0..9 {
        let p = (
            cx + rng.range(-0.18, 0.18) * s,
            h * (0.36 + 0.05 * i as f32),
        );
        let rad = s * rng.range(0.18, 0.26);
        r.part(
            Bounds::around(p.0, p.1, rad * 1.2),
            blob(rng.next_u64(), p.0, p.1, rad, 0.2),
            straw(seed ^ i, HAY, (0.3, 0.95)),
            dome(12.0 + i as f32, 12.0, rad),
            SOFT_INK,
        );
    }
}
