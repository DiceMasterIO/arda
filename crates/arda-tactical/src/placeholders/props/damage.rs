//! Damage props (1.0 task 05): the lasting results of a spell or a siege
//! that layouts and bakes may place, a heap of rubble, a broken deck and a
//! charred floor patch.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::{plank_rect, DARK_WOOD, EAST, PALE_WOOD, SOUTH, STONE};
use crate::placeholders::material::{painted, stone, Rng};
use crate::placeholders::paint::{blob, tone};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, NO_INK};

/// Rubble heap (2 × 2): a mound of broken masonry with a few splintered beams.
pub fn rubble_heap(r: &mut Relief, seed: u64, _s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    for i in 0..44u64 {
        let ring = if i < 26 { 0.34 } else { 0.18 };
        let (dx, dy) = rng.dir();
        let d = rng.range(0.0, ring);
        let c = (w * (0.5 + dx * d), h * (0.5 + dy * d));
        let rad = w * rng.range(0.035, 0.07);
        let col = tone(STONE, rng.range(0.74, 1.12));
        r.part(
            Bounds::around(c.0, c.1, rad * 1.4),
            blob(seed ^ i, c.0, c.1, rad, 0.35),
            stone(seed ^ (i * 7), col),
            dome(i as f32 * 0.5, rad * 0.8, rad),
            INK,
        );
    }
    for k in 0..3u64 {
        let (dx, dy) = rng.dir();
        let c = (w * (0.5 + dx * 0.18), h * (0.5 + dy * 0.18));
        let dir = if k % 2 == 0 { EAST } else { SOUTH };
        let half = (w * rng.range(0.08, 0.14), w * 0.018);
        let half = if k % 2 == 0 { half } else { (half.1, half.0) };
        plank_rect(r, seed ^ (0xB0 + k), c, half, DARK_WOOD, dir, 0.0, 14.0);
    }
}

/// Broken deck (1 × 2): planks with a gap torn in the middle and splinters.
pub fn broken_deck(r: &mut Relief, seed: u64, _s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let board = w / 8.0;
    let mut rng = Rng::new(seed);
    for i in 0..8 {
        let x = board * (i as f32 + 0.5);
        // Each board survives from an end up to a ragged break.
        let from_top = i % 2 == 0;
        let reach = h * rng.range(0.22, 0.46);
        let y = if from_top {
            reach / 2.0
        } else {
            h - reach / 2.0
        };
        let col = tone(PALE_WOOD, rng.range(0.78, 1.02));
        plank_rect(
            r,
            rng.next_u64(),
            (x, y),
            (board / 2.0 - 1.0, reach / 2.0),
            col,
            SOUTH,
            0.0,
            0.0,
        );
        let sx = x + rng.range(-board * 0.3, board * 0.3);
        let sy = h * rng.range(0.4, 0.6);
        let rad = board * rng.range(0.14, 0.24);
        r.part(
            Bounds::around(sx, sy, rad * 1.4),
            blob(seed ^ (i as u64 + 31), sx, sy, rad, 0.5),
            painted(seed ^ i as u64, tone(PALE_WOOD, 0.7), 4.0),
            bevel(0.0, 3.0, rad),
            INK,
        );
    }
}

/// Charred floor (2 × 2): a scorched, cracked patch with a few embers.
pub fn charred_floor(r: &mut Relief, seed: u64, _s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    r.part(
        Bounds::around(w / 2.0, h / 2.0, w * 0.46),
        blob(seed, w / 2.0, h / 2.0, w * 0.4, 0.22),
        painted(seed ^ 1, [34, 30, 28], w / 10.0),
        bevel(0.0, 1.5, w * 0.1),
        NO_INK,
    );
    for i in 0..16u64 {
        let (dx, dy) = rng.dir();
        let d = rng.range(0.0, 0.3);
        let c = (w * (0.5 + dx * d), h * (0.5 + dy * d));
        let rad = w * rng.range(0.02, 0.05);
        let hot = i % 5 == 0;
        let col = if hot { [196, 74, 22] } else { [18, 16, 15] };
        r.part(
            Bounds::around(c.0, c.1, rad * 1.5),
            blob(seed ^ (i + 77), c.0, c.1, rad, 0.4),
            painted(seed ^ i, col, 3.0),
            bevel(0.0, 1.5, rad),
            NO_INK,
        );
    }
}
