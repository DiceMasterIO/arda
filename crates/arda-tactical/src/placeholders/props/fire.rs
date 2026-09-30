//! Fire and light: braziers, hearths, ovens, forges, lanterns and candle
//! stands. Flames and coals are painted bright and flat; the light they give
//! comes from each asset's `light` record in the lighting pass.

use super::{
    disc, glow, iron_rect, plank_rect, pole, puck, rect, ring, stone_rect, DARK_WOOD, EAST, IRON,
    STONE,
};
use crate::placeholders::material::{metal, painted, solid, stone, wood, Rng};
use crate::placeholders::paint::{mix, tone};
use crate::placeholders::relief::{Relief, INK, SOFT_INK};

/// Brazier: an iron bowl of glowing coals on three feet.
pub fn brazier(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    for (dx, dy) in [(0.0, -0.24), (0.21, 0.12), (-0.21, 0.12)] {
        disc(
            r,
            (c.0 + dx * s, c.1 + dy * s),
            s * 0.035,
            metal(seed, IRON),
            0.0,
            2.0,
            INK,
        );
    }
    disc(r, c, s * 0.22, metal(seed, IRON), 2.0, 6.0, INK);
    glow(r, seed, c, s * 0.16, 8.0);
    for i in 0..5 {
        let mut rng = Rng::new(seed ^ i);
        let p = (
            c.0 + rng.range(-0.1, 0.1) * s,
            c.1 + rng.range(-0.1, 0.1) * s,
        );
        disc(r, p, s * 0.03, solid([60, 40, 30]), 9.0, 1.0, SOFT_INK);
    }
}

/// Hearth (2 × 1): a stone fireplace open to the south with a log fire.
pub fn hearth(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    // Back wall and cheeks of rough stones.
    for i in 0..9 {
        let x = w * (0.1 + 0.1 * i as f32);
        let col = tone(STONE, rng.range(0.85, 1.1));
        stone_rect(
            r,
            rng.next_u64(),
            (x, h * 0.2),
            (s * 0.07, s * 0.12),
            col,
            8.0,
        );
    }
    for (x, y) in [(0.1, 0.45), (0.1, 0.68), (0.9, 0.45), (0.9, 0.68)] {
        let col = tone(STONE, rng.range(0.85, 1.1));
        stone_rect(
            r,
            rng.next_u64(),
            (w * x, h * y),
            (s * 0.09, s * 0.1),
            col,
            8.0,
        );
    }
    rect(
        r,
        (w / 2.0, h * 0.55),
        (w * 0.3, h * 0.3),
        4.0,
        painted(seed, [50, 40, 34], 5.0),
        0.0,
        1.0,
        SOFT_INK,
    );
    pole(
        r,
        seed ^ 1,
        (w * 0.32, h * 0.62),
        (w * 0.62, h * 0.46),
        s * 0.05,
        [96, 66, 40],
        3.0,
    );
    pole(
        r,
        seed ^ 2,
        (w * 0.38, h * 0.44),
        (w * 0.68, h * 0.62),
        s * 0.05,
        [110, 76, 46],
        5.0,
    );
    glow(r, seed, (w / 2.0, h * 0.54), s * 0.2, 10.0);
}

/// Oven: a clay bread oven dome with a glowing mouth to the south.
pub fn oven(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.46);
    disc(
        r,
        c,
        s * 0.42,
        painted(seed, [176, 120, 82], 6.0),
        0.0,
        s * 0.3,
        INK,
    );
    disc(
        r,
        (c.0, c.1 - s * 0.05),
        s * 0.07,
        stone(seed ^ 1, [90, 80, 72]),
        s * 0.3,
        4.0,
        INK,
    );
    rect(
        r,
        (c.0, c.1 + s * 0.38),
        (s * 0.13, s * 0.07),
        5.0,
        solid([40, 28, 22]),
        4.0,
        1.0,
        INK,
    );
    glow(r, seed ^ 2, (c.0, c.1 + s * 0.38), s * 0.07, 6.0);
}

/// Forge (2 × 1): a stone hearth of glowing coals with leather bellows.
pub fn forge(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    stone_rect(
        r,
        seed,
        (w * 0.4, h / 2.0),
        (s * 0.66, s * 0.38),
        [128, 120, 110],
        0.0,
    );
    rect(
        r,
        (w * 0.4, h / 2.0),
        (s * 0.46, s * 0.24),
        6.0,
        painted(seed, [44, 38, 34], 4.0),
        6.0,
        1.0,
        SOFT_INK,
    );
    glow(r, seed, (w * 0.4, h / 2.0), s * 0.2, 8.0);
    let bel = (w * 0.86, h / 2.0);
    disc(
        r,
        bel,
        s * 0.17,
        painted(seed ^ 3, [120, 80, 50], 4.0),
        2.0,
        8.0,
        INK,
    );
    plank_rect(
        r,
        seed ^ 4,
        (bel.0 - s * 0.14, bel.1),
        (s * 0.08, s * 0.03),
        DARK_WOOD,
        EAST,
        0.0,
        10.0,
    );
    iron_rect(r, seed, (w * 0.73, h / 2.0), (s * 0.06, 2.0), 10.0);
}

/// Lantern: an iron frame around a glowing pane.
pub fn lantern(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    rect(
        r,
        c,
        (s * 0.12, s * 0.12),
        2.0,
        metal(seed, [60, 58, 56]),
        0.0,
        3.0,
        INK,
    );
    rect(
        r,
        c,
        (s * 0.085, s * 0.085),
        1.0,
        |x, y, _| mix([255, 232, 150], [236, 150, 60], ((x + y) / 90.0).fract()),
        4.0,
        0.0,
        SOFT_INK,
    );
    ring(r, c, s * 0.035, s * 0.05, metal(seed ^ 1, IRON), 6.0);
}

/// Candle stand: an iron tripod with three candles.
pub fn candle_stand(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    for (dx, dy) in [(0.0, -0.2), (0.18, 0.11), (-0.18, 0.11)] {
        pole(r, seed, c, (c.0 + dx * s, c.1 + dy * s), 1.6, IRON, 2.0);
        puck(
            r,
            (c.0 + dx * s, c.1 + dy * s),
            s * 0.06,
            metal(seed, IRON),
            4.0,
            INK,
        );
        puck(
            r,
            (c.0 + dx * s, c.1 + dy * s),
            s * 0.04,
            solid([236, 228, 206]),
            7.0,
            SOFT_INK,
        );
        glow(r, seed, (c.0 + dx * s, c.1 + dy * s), s * 0.018, 10.0);
    }
    disc(r, c, s * 0.05, metal(seed ^ 2, IRON), 4.0, 3.0, INK);
    let _ = wood;
}
