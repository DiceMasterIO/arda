//! Road and river-crossing props: milestones, marker posts, ferry ropes,
//! ferry boats and stone bridge decks.

use super::{
    disc, plank_rect, pole, puck, rect, stone_rect, DARK_WOOD, EAST, PALE_WOOD, SOUTH, WOOD,
};
use crate::noise::{hash2, unit};
use crate::placeholders::material::{cloth, solid, stone, wood, Rng};
use crate::placeholders::paint::{rbox, tone};
use crate::placeholders::relief::{bevel, Bounds, Relief, INK, SOFT_INK};

/// Milestone: a squat stone post with a rounded top and carved notches.
pub fn milestone(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    stone_rect(r, seed, c, (s * 0.16, s * 0.12), [168, 162, 148], 0.0);
    rect(
        r,
        (c.0, c.1 - s * 0.02),
        (s * 0.12, s * 0.07),
        s * 0.06,
        stone(seed ^ 1, [182, 176, 162]),
        6.0,
        5.0,
        INK,
    );
    for dx in [-0.05, 0.0, 0.05] {
        rect(
            r,
            (c.0 + dx * s, c.1 + s * 0.07),
            (1.2, s * 0.025),
            0.5,
            solid([96, 92, 84]),
            7.0,
            0.0,
            SOFT_INK,
        );
    }
}

/// Marker post: a stake with a painted red band and a small sign board.
pub fn marker_post(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    plank_rect(
        r,
        seed,
        (c.0 + s * 0.12, c.1 - s * 0.02),
        (s * 0.13, s * 0.05),
        PALE_WOOD,
        EAST,
        0.0,
        10.0,
    );
    puck(r, c, s * 0.075, wood(seed ^ 1, WOOD, EAST, 0.0), 14.0, INK);
    puck(
        r,
        c,
        s * 0.05,
        cloth(seed ^ 2, [170, 44, 38], EAST),
        17.0,
        SOFT_INK,
    );
}

/// Ferry rope: a twisted hemp rope strung along the square's centre line.
pub fn ferry_rope(r: &mut Relief, seed: u64, s: f32) {
    let c = s / 2.0;
    let rad = s * 0.022;
    r.part(
        Bounds(0.0, c - rad - 3.0, s, c + rad + 3.0),
        rbox(c, c, s / 2.0, rad, rad),
        move |x, y, _| {
            // A twist: diagonal bands every few pixels.
            let band = ((x + (y - c) * 1.6) / 4.0).floor();
            let k = if band.rem_euclid(2.0) < 1.0 {
                1.0
            } else {
                0.82
            };
            tone(
                [196, 170, 118],
                k * (0.94 + 0.12 * unit(hash2(seed, band as i64, 0))),
            )
        },
        bevel(20.0, 2.0, rad),
        INK,
    );
}

/// Ferry boat (2 × 3): a flat planked barge with rails and rope guides.
pub fn ferry_boat(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    r.part(
        r.all(),
        rbox(c.0, c.1, w * 0.46, h * 0.47, s * 0.2),
        wood(seed, DARK_WOOD, SOUTH, 0.0),
        bevel(0.0, 6.0, 5.0),
        INK,
    );
    let mut rng = Rng::new(seed);
    let boards = 9;
    let bw = (w * 0.8) / boards as f32;
    for i in 0..boards {
        let x = c.0 - w * 0.4 + bw * (i as f32 + 0.5);
        let col = tone(PALE_WOOD, rng.range(0.82, 1.06));
        plank_rect(
            r,
            rng.next_u64(),
            (x, c.1),
            (bw / 2.0 - 0.8, h * 0.4),
            col,
            SOUTH,
            0.0,
            6.0,
        );
    }
    for x in [c.0 - w * 0.43, c.0 + w * 0.43] {
        plank_rect(
            r,
            seed ^ 3,
            (x, c.1),
            (s * 0.05, h * 0.42),
            WOOD,
            SOUTH,
            0.0,
            12.0,
        );
    }
    for y in [c.1 - h * 0.44, c.1 + h * 0.44] {
        disc(
            r,
            (c.0, y),
            s * 0.07,
            wood(seed ^ 4, DARK_WOOD, EAST, 0.0),
            14.0,
            5.0,
            INK,
        );
        pole(
            r,
            seed ^ 5,
            (c.0 - s * 0.05, y),
            (c.0 + s * 0.05, y),
            2.0,
            [196, 170, 118],
            20.0,
        );
    }
}

/// Stone bridge deck: one square of paving, a 3 × 3 set of dressed slabs
/// in mortar, tiling edge to edge.
pub fn bridge_deck_stone(r: &mut Relief, seed: u64, s: f32) {
    let c = s / 2.0;
    r.part(
        r.all(),
        rbox(c, c, c, c, 2.0),
        stone(seed, [112, 106, 96]),
        bevel(0.0, 1.0, 2.0),
        super::super::relief::NO_INK,
    );
    let cell = s / 3.0;
    for j in 0..3 {
        for i in 0..3 {
            let jitter = unit(hash2(seed, i, j));
            let base = tone([170, 164, 150], 0.88 + 0.2 * jitter);
            let (x, y) = (cell * (i as f32 + 0.5), cell * (j as f32 + 0.5));
            stone_rect(
                r,
                seed ^ (i * 3 + j) as u64,
                (x, y),
                (cell / 2.0 - 1.6, cell / 2.0 - 1.6),
                base,
                2.0,
            );
        }
    }
}
