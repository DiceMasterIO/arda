//! Furniture: tables, benches, beds, chairs, stools, bar counters, pews,
//! thrones, small rugs and banners.

use super::{
    disc, iron_rect, plank_rect, pole, puck, rect, BRASS, DARK_WOOD, EAST, PALE_WOOD, RED, SOUTH,
    WOOD,
};
use crate::placeholders::material::{cloth, metal, solid, wood, Rng};
use crate::placeholders::paint::{rbox, tone};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, NO_INK, SOFT_INK};

/// Table (2 × 1): planks along its length, a mug, a plate and bread.
pub fn table(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let (hw, hh) = (w / 2.0 - s * 0.1, h / 2.0 - s * 0.16);
    plank_rect(r, seed, c, (hw, hh), PALE_WOOD, EAST, hh * 2.0 / 4.0, 0.0);
    disc(
        r,
        (c.0 - s * 0.45, c.1),
        s * 0.08,
        solid([206, 200, 186]),
        4.0,
        1.5,
        INK,
    );
    disc(
        r,
        (c.0 - s * 0.45, c.1),
        s * 0.05,
        solid([176, 150, 100]),
        5.5,
        2.0,
        SOFT_INK,
    );
    puck(
        r,
        (c.0 + s * 0.35, c.1 - s * 0.08),
        s * 0.05,
        wood(seed ^ 5, WOOD, SOUTH, 0.0),
        4.0,
        INK,
    );
    disc(
        r,
        (c.0 + s * 0.35, c.1 - s * 0.08),
        s * 0.032,
        solid([96, 60, 30]),
        6.0,
        0.5,
        NO_INK,
    );
    disc(
        r,
        (c.0 + s * 0.05, c.1 + s * 0.06),
        s * 0.07,
        solid([188, 132, 64]),
        4.0,
        5.0,
        INK,
    );
}

/// Bench (2 × 1): a seat of two boards.
pub fn bench(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    plank_rect(
        r,
        seed,
        (w / 2.0, h / 2.0),
        (w / 2.0 - s * 0.08, s * 0.13),
        WOOD,
        EAST,
        s * 0.13,
        0.0,
    );
}

/// Bed (1 × 2): frame, blanket, a folded sheet and a pillow at the head.
pub fn bed(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    plank_rect(
        r,
        seed,
        (cx, h / 2.0),
        (s * 0.4, h / 2.0 - s * 0.06),
        DARK_WOOD,
        SOUTH,
        0.0,
        0.0,
    );
    rect(
        r,
        (cx, h / 2.0 + s * 0.14),
        (s * 0.35, h / 2.0 - s * 0.28),
        5.0,
        cloth(seed ^ 3, [138, 52, 44], EAST),
        5.0,
        5.0,
        INK,
    );
    rect(
        r,
        (cx, s * 0.62),
        (s * 0.35, s * 0.08),
        3.0,
        cloth(seed ^ 4, [222, 216, 196], EAST),
        10.0,
        2.0,
        SOFT_INK,
    );
    rect(
        r,
        (cx, s * 0.3),
        (s * 0.27, s * 0.13),
        7.0,
        cloth(seed ^ 5, [232, 228, 212], EAST),
        6.0,
        7.0,
        INK,
    );
}

/// Chair: a square seat with a raised backrest on its north side.
pub fn chair(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0 + s * 0.03);
    plank_rect(r, seed, c, (s * 0.2, s * 0.19), WOOD, EAST, s * 0.1, 0.0);
    plank_rect(
        r,
        seed ^ 1,
        (c.0, c.1 - s * 0.2),
        (s * 0.22, s * 0.045),
        DARK_WOOD,
        EAST,
        0.0,
        14.0,
    );
}

/// Stool: a round seat.
pub fn stool(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    puck(r, c, s * 0.17, wood(seed, WOOD, EAST, s * 0.08), 0.0, INK);
}

/// Bar counter (2 × 1): a polished top with a brass rail, mugs and a rag.
pub fn bar_counter(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h * 0.45);
    plank_rect(
        r,
        seed,
        c,
        (w / 2.0 - s * 0.04, s * 0.24),
        DARK_WOOD,
        EAST,
        s * 0.12,
        0.0,
    );
    rect(
        r,
        (c.0, c.1 + s * 0.28),
        (w / 2.0 - s * 0.08, 2.2),
        1.0,
        metal(seed, BRASS),
        2.0,
        1.5,
        SOFT_INK,
    );
    let mut rng = Rng::new(seed);
    for x in [0.25, 0.42, 1.3, 1.62] {
        let p = (x * s + rng.range(-3.0, 3.0), c.1 + rng.range(-8.0, 8.0));
        puck(
            r,
            p,
            s * 0.05,
            wood(rng.next_u64(), PALE_WOOD, SOUTH, 0.0),
            5.0,
            INK,
        );
        disc(r, p, s * 0.033, solid([214, 196, 150]), 7.0, 1.0, NO_INK);
    }
    rect(
        r,
        (s * 0.95, c.1 + 3.0),
        (s * 0.1, s * 0.06),
        3.0,
        cloth(seed ^ 7, [206, 198, 176], EAST),
        5.0,
        2.0,
        SOFT_INK,
    );
}

/// Pew (2 × 1): a long bench with a backrest along its north edge.
pub fn pew(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    plank_rect(
        r,
        seed,
        (w / 2.0, h / 2.0 + s * 0.06),
        (w / 2.0 - s * 0.06, s * 0.16),
        WOOD,
        EAST,
        s * 0.16,
        0.0,
    );
    plank_rect(
        r,
        seed ^ 1,
        (w / 2.0, h / 2.0 - s * 0.14),
        (w / 2.0 - s * 0.04, s * 0.05),
        DARK_WOOD,
        EAST,
        0.0,
        14.0,
    );
    for x in [s * 0.08, w - s * 0.08] {
        plank_rect(
            r,
            seed ^ 2,
            (x, h / 2.0),
            (s * 0.05, s * 0.22),
            DARK_WOOD,
            SOUTH,
            0.0,
            16.0,
        );
    }
}

/// Throne: a high carved back, armrests and a red cushion.
pub fn throne(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0 + s * 0.05);
    plank_rect(r, seed, c, (s * 0.34, s * 0.3), DARK_WOOD, SOUTH, 0.0, 0.0);
    rect(
        r,
        (c.0, c.1 + s * 0.02),
        (s * 0.22, s * 0.2),
        6.0,
        cloth(seed ^ 3, RED, EAST),
        6.0,
        6.0,
        INK,
    );
    for dx in [-0.28, 0.28] {
        plank_rect(
            r,
            seed ^ 4,
            (c.0 + dx * s, c.1 + s * 0.04),
            (s * 0.06, s * 0.24),
            DARK_WOOD,
            SOUTH,
            0.0,
            12.0,
        );
    }
    plank_rect(
        r,
        seed ^ 5,
        (c.0, c.1 - s * 0.3),
        (s * 0.36, s * 0.07),
        DARK_WOOD,
        EAST,
        0.0,
        20.0,
    );
    for dx in [-0.3, 0.0, 0.3] {
        disc(
            r,
            (c.0 + dx * s, c.1 - s * 0.3),
            s * 0.05,
            metal(seed ^ 6, BRASS),
            26.0,
            4.0,
            INK,
        );
    }
}

/// Small rug (2 × 1): a bordered woven rug with fringed ends.
pub fn rug_small(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let (hw, hh) = (w / 2.0 - s * 0.14, h / 2.0 - s * 0.12);
    for i in 0..14 {
        let y = c.1 - hh + (i as f32 + 0.5) * (2.0 * hh / 14.0);
        for x in [c.0 - hw - s * 0.06, c.0 + hw + s * 0.06] {
            let x0 = if x < c.0 { c.0 - hw } else { c.0 + hw };
            pole(r, seed, (x0, y), (x, y), 1.0, [206, 192, 160], 0.0);
        }
    }
    let base = tone([128, 44, 40], 1.0);
    r.part(
        Bounds::around(c.0, c.1, hw),
        rbox(c.0, c.1, hw, hh, 2.0),
        move |x, y, d| {
            let e = -d;
            let (u, v) = ((x - c.0) / hw, (y - c.1) / hh);
            let col = if e < s * 0.05 {
                [44, 50, 86]
            } else if e < s * 0.08 {
                [204, 170, 90]
            } else if (u.abs() * 1.6 + v.abs()) < 0.6 {
                [44, 50, 86]
            } else if (u.abs() * 1.6 + v.abs()) < 0.75 {
                [220, 200, 160]
            } else {
                base
            };
            cloth(seed, col, EAST)(x, y, d)
        },
        bevel(0.0, 1.5, 2.0),
        SOFT_INK,
    );
}

/// Banner: a standard on a crossbar, cloth hanging to the south.
pub fn banner(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.3);
    rect(
        r,
        (c.0, c.1 + s * 0.2),
        (s * 0.18, s * 0.22),
        1.0,
        move |x, y, d| {
            let emblem = ((x - s / 2.0).abs() + (y - s * 0.52).abs()) < s * 0.08;
            cloth(seed, if emblem { [214, 176, 70] } else { RED }, SOUTH)(x, y, d)
        },
        4.0,
        2.0,
        INK,
    );
    pole(
        r,
        seed,
        (c.0 - s * 0.24, c.1),
        (c.0 + s * 0.24, c.1),
        2.2,
        DARK_WOOD,
        10.0,
    );
    disc(
        r,
        c,
        s * 0.05,
        wood(seed ^ 1, DARK_WOOD, EAST, 0.0),
        12.0,
        4.0,
        INK,
    );
    iron_rect(r, seed, (c.0, c.1), (1.5, 1.5), 16.0);
    let _ = dome;
}
