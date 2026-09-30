//! Civic and religious props: signposts, graves, altars and statues.

use super::{disc, puck, rect, stone_rect, DARK_WOOD, EAST, PALE_WOOD, SOUTH};
use crate::placeholders::material::{cloth, painted, solid, stone, wood, Rng};
use crate::placeholders::paint::union;
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

/// Signpost: a post with three pointing arms.
pub fn signpost(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    for (i, (dx, dy)) in [(0.95f32, -0.3f32), (-0.8, -0.6), (0.2, 0.98)]
        .into_iter()
        .enumerate()
    {
        let l = (dx * dx + dy * dy).sqrt();
        let (ux, uy) = (dx / l, dy / l);
        let tip = (c.0 + ux * s * 0.38, c.1 + uy * s * 0.38);
        let arm = crate::placeholders::paint::obox(
            c.0 + ux * s * 0.2,
            c.1 + uy * s * 0.2,
            s * 0.19,
            s * 0.05,
            1.0,
            (ux, uy),
        );
        let point = crate::placeholders::paint::obox(
            tip.0,
            tip.1,
            s * 0.04,
            s * 0.04,
            0.5,
            (0.707 * (ux - uy), 0.707 * (ux + uy)),
        );
        r.part(
            r.all(),
            union(arm, point),
            wood(seed ^ i as u64, PALE_WOOD, (ux, uy), 0.0),
            bevel(10.0 + i as f32 * 3.0, 2.0, 2.0),
            INK,
        );
    }
    puck(r, c, s * 0.07, wood(seed, DARK_WOOD, EAST, 0.0), 18.0, INK);
}

/// Grave: an earth mound with grass tufts and a headstone to the north.
pub fn grave(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.58);
    rect(
        r,
        c,
        (s * 0.2, s * 0.32),
        s * 0.18,
        painted(seed, [104, 84, 58], 5.0),
        0.0,
        8.0,
        INK,
    );
    let mut rng = Rng::new(seed);
    for _ in 0..7 {
        let p = (
            c.0 + rng.range(-0.15, 0.15) * s,
            c.1 + rng.range(-0.25, 0.25) * s,
        );
        disc(
            r,
            p,
            s * rng.range(0.025, 0.045),
            painted(rng.next_u64(), [96, 118, 52], 3.0),
            8.0,
            2.0,
            SOFT_INK,
        );
    }
    stone_rect(
        r,
        seed ^ 1,
        (c.0, s * 0.18),
        (s * 0.24, s * 0.07),
        [150, 148, 140],
        6.0,
    );
}

/// Altar (2 × 1): a stone block with a cloth runner, candles and a book.
pub fn altar(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    stone_rect(r, seed, c, (w * 0.44, s * 0.3), [186, 180, 166], 0.0);
    rect(
        r,
        c,
        (s * 0.2, s * 0.32),
        1.0,
        cloth(seed, [120, 30, 36], SOUTH),
        6.0,
        1.0,
        SOFT_INK,
    );
    rect(
        r,
        (c.0, c.1 - s * 0.02),
        (s * 0.12, s * 0.09),
        1.0,
        cloth(seed ^ 2, [220, 206, 170], EAST),
        8.0,
        2.0,
        INK,
    );
    for x in [w * 0.16, w * 0.84] {
        puck(r, (x, c.1), s * 0.05, solid([236, 228, 206]), 7.0, INK);
        super::glow(r, seed, (x, c.1), s * 0.02, 10.0);
    }
}

/// Statue: a robed stone figure on a square pedestal, seen from above.
pub fn statue(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    stone_rect(r, seed, c, (s * 0.42, s * 0.42), [160, 156, 146], 0.0);
    let figure = stone(seed ^ 1, [176, 172, 162]);
    r.part(
        Bounds::around(c.0, c.1, s * 0.35),
        crate::placeholders::paint::ellipse(c.0, c.1 + s * 0.02, s * 0.28, s * 0.14),
        figure,
        dome(8.0, 20.0, s * 0.14),
        INK,
    );
    disc(
        r,
        (c.0, c.1 - s * 0.01),
        s * 0.1,
        stone(seed ^ 2, [184, 180, 170]),
        30.0,
        10.0,
        INK,
    );
    for dx in [-0.22, 0.22] {
        disc(
            r,
            (c.0 + dx * s, c.1 + s * 0.08),
            s * 0.05,
            stone(seed ^ 3, [170, 166, 156]),
            22.0,
            4.0,
            INK,
        );
    }
}
