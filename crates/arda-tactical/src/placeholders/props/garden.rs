//! Kitchen-garden pieces (1.0 task 17): vegetable and herb beds, bean poles,
//! a cold frame, a compost heap, a rain barrel and floor-level garden tools.

use super::{
    disc, plank_rect, pole, puck, rect, ring, DARK_WOOD, EAST, IRON, PALE_WOOD, SOUTH, WOOD,
};
use crate::placeholders::material::{leafy, metal, painted, solid, wood, Rng};
use crate::placeholders::paint::{blob, mix, rbox, tone};
use crate::placeholders::relief::{bevel, dome, level, Bounds, Relief, INK, NO_INK, SOFT_INK};

/// Turned soil in the dug bed.
const SOIL: [u8; 3] = [96, 66, 44];

/// The dug soil of a bed with a low raised edge.
fn bed_frame(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    r.part(
        Bounds::around(c.0, c.1, s * 0.46),
        rbox(c.0, c.1, s * 0.44, s * 0.44, 3.0),
        painted(seed, SOIL, 3.0),
        bevel(0.0, 2.0, 5.0),
        INK,
    );
}

/// Vegetable bed (1 x 1): a dug bed of cabbages, leeks and onions in rows.
pub fn vegetable_bed(r: &mut Relief, seed: u64, s: f32) {
    bed_frame(r, seed, s);
    let mut rng = Rng::new(seed ^ 0x5E6);
    let leaf = [[104, 146, 72], [128, 168, 96], [76, 120, 64]];
    for row in 0..3 {
        let y = s * (0.24 + 0.26 * row as f32);
        for col in 0..3 {
            let x = s * (0.24 + 0.26 * col as f32) + rng.range(-1.5, 1.5);
            let rad = if row == 1 { s * 0.085 } else { s * 0.06 };
            let base = leaf[(row + col) % 3];
            r.part(
                Bounds::around(x, y, rad * 1.3),
                blob(rng.next_u64(), x, y, rad, 0.2),
                leafy(rng.next_u64(), base, 2.5),
                dome(2.0, rad * 0.7, rad),
                SOFT_INK,
            );
        }
    }
}

/// Herb bed (1 x 1): a dug bed of small mixed clumps, some flowering.
pub fn herb_bed(r: &mut Relief, seed: u64, s: f32) {
    bed_frame(r, seed, s);
    let mut rng = Rng::new(seed ^ 0x4E7B);
    let tones = [
        [120, 150, 84],
        [86, 128, 92],
        [150, 168, 110],
        [104, 120, 84],
    ];
    for _ in 0..9 {
        let p = (s * rng.range(0.2, 0.8), s * rng.range(0.2, 0.8));
        let rad = s * rng.range(0.05, 0.09);
        let base = rng.pick(&tones).unwrap_or(tones[0]);
        r.part(
            Bounds::around(p.0, p.1, rad * 1.3),
            blob(rng.next_u64(), p.0, p.1, rad, 0.25),
            leafy(rng.next_u64(), base, 2.0),
            dome(2.0, rad * 0.6, rad),
            SOFT_INK,
        );
    }
    for _ in 0..6 {
        let p = (s * rng.range(0.2, 0.8), s * rng.range(0.2, 0.8));
        let col = if rng.f() < 0.5 {
            [176, 120, 196]
        } else {
            [236, 234, 222]
        };
        r.part(
            Bounds::around(p.0, p.1, 3.0),
            blob(rng.next_u64(), p.0, p.1, 2.0, 0.2),
            solid(col),
            dome(7.0, 1.5, 2.0),
            NO_INK,
        );
    }
}

/// Bean poles (1 x 1): crossed canes in two rows with climbing beans.
pub fn bean_poles(r: &mut Relief, seed: u64, s: f32) {
    bed_frame(r, seed, s);
    let mut rng = Rng::new(seed ^ 0xBEA);
    for x in [0.3, 0.7] {
        // Two canes leaning together to a crossing at the north-south spine.
        pole(
            r,
            seed,
            (s * x - s * 0.05, s * 0.12),
            (s * x + s * 0.05, s * 0.88),
            1.6,
            PALE_WOOD,
            4.0,
        );
        pole(
            r,
            seed ^ 1,
            (s * x + s * 0.05, s * 0.12),
            (s * x - s * 0.05, s * 0.88),
            1.6,
            PALE_WOOD,
            5.0,
        );
    }
    pole(
        r,
        seed ^ 2,
        (s * 0.3, s * 0.5),
        (s * 0.7, s * 0.5),
        1.6,
        PALE_WOOD,
        12.0,
    );
    for i in 0..22 {
        let x = s * (if i % 2 == 0 { 0.3 } else { 0.7 }) + rng.range(-4.0, 4.0);
        let y = s * rng.range(0.14, 0.86);
        let rad = s * rng.range(0.04, 0.07);
        r.part(
            Bounds::around(x, y, rad * 1.3),
            blob(rng.next_u64(), x, y, rad, 0.3),
            leafy(rng.next_u64(), [84, 134, 62], 2.0),
            dome(7.0, rad * 0.5, rad),
            SOFT_INK,
        );
    }
    for _ in 0..7 {
        let x = s * (if rng.f() < 0.5 { 0.3 } else { 0.7 }) + rng.range(-3.0, 3.0);
        let y = s * rng.range(0.2, 0.8);
        r.part(
            Bounds::around(x, y, 3.0),
            blob(rng.next_u64(), x, y, 1.8, 0.2),
            solid([206, 62, 54]),
            dome(10.0, 1.5, 2.0),
            NO_INK,
        );
    }
}

/// Cold frame (2 x 1): a low plank box with a glass lid propped open.
pub fn cold_frame(r: &mut Relief, seed: u64, _s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    plank_rect(r, seed, c, (w * 0.46, h * 0.4), WOOD, EAST, 0.0, 0.0);
    r.part(
        Bounds::around(c.0, c.1, w * 0.4),
        rbox(c.0, c.1, w * 0.4, h * 0.32, 2.0),
        painted(seed ^ 3, SOIL, 3.0),
        level(3.0),
        SOFT_INK,
    );
    let (hw, hh) = (w * 0.42, h * 0.34);
    r.part(
        Bounds::around(c.0, c.1, hw),
        rbox(c.0, c.1, hw, hh, 2.0),
        |x, y, _| {
            mix(
                [150, 190, 196],
                [214, 234, 236],
                ((x * 0.4 + y * 0.8) / 40.0).fract(),
            )
        },
        bevel(8.0, 3.0, 3.0),
        INK,
    );
    for k in 1..3 {
        let x = c.0 - hw + 2.0 * hw * k as f32 / 3.0;
        pole(r, seed, (x, c.1 - hh), (x, c.1 + hh), 1.2, DARK_WOOD, 12.0);
    }
    pole(
        r,
        seed,
        (c.0 - hw, c.1),
        (c.0 + hw, c.1),
        1.2,
        DARK_WOOD,
        12.5,
    );
}

/// Compost heap (1 x 1): a dark mound of rotting greens and straw.
pub fn compost_heap(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let mut rng = Rng::new(seed ^ 0xC0A);
    r.part(
        Bounds::around(c.0, c.1, s * 0.4),
        blob(seed, c.0, c.1, s * 0.36, 0.22),
        painted(seed, [70, 54, 36], 3.0),
        dome(0.0, 14.0, s * 0.36),
        INK,
    );
    let bits = [
        [120, 140, 70],
        [196, 170, 96],
        [150, 100, 60],
        [96, 124, 64],
    ];
    for _ in 0..22 {
        let (dx, dy) = rng.dir();
        let off = s * 0.26 * rng.f().sqrt();
        let p = (c.0 + dx * off, c.1 + dy * off);
        let col = tone(rng.pick(&bits).unwrap_or(bits[0]), rng.range(0.8, 1.1));
        r.part(
            Bounds::around(p.0, p.1, 4.0),
            blob(rng.next_u64(), p.0, p.1, rng.range(1.5, 3.0), 0.3),
            solid(col),
            dome(8.0, 1.5, 3.0),
            NO_INK,
        );
    }
}

/// Rain barrel (1 x 1): an open staved barrel full of water, iron hooped.
pub fn rain_barrel(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    puck(r, c, s * 0.36, wood(seed, WOOD, SOUTH, s * 0.05), 0.0, INK);
    disc(
        r,
        c,
        s * 0.3,
        |x, y, _| {
            mix(
                [40, 72, 88],
                [90, 130, 140],
                ((x * 0.6 + y) / 50.0).fract() * 0.7,
            )
        },
        8.0,
        0.5,
        SOFT_INK,
    );
    ring(r, c, s * 0.3, s * 0.34, metal(seed, IRON), 10.0);
    ring(r, c, s * 0.2, s * 0.215, metal(seed ^ 1, IRON), 9.0);
}

/// Garden tools (1 x 1): a spade, a hoe and a rake lying on the ground.
pub fn garden_tools(r: &mut Relief, seed: u64, s: f32) {
    // Spade: shaft with a steel blade.
    pole(
        r,
        seed,
        (s * 0.2, s * 0.14),
        (s * 0.28, s * 0.66),
        1.6,
        WOOD,
        2.0,
    );
    rect(
        r,
        (s * 0.3, s * 0.76),
        (s * 0.07, s * 0.1),
        3.0,
        metal(seed, IRON),
        1.0,
        2.0,
        INK,
    );
    // Hoe: shaft with a flat blade across the end.
    pole(
        r,
        seed ^ 1,
        (s * 0.5, s * 0.1),
        (s * 0.52, s * 0.72),
        1.6,
        WOOD,
        3.0,
    );
    rect(
        r,
        (s * 0.52, s * 0.76),
        (s * 0.08, s * 0.03),
        1.5,
        metal(seed ^ 1, IRON),
        2.0,
        2.0,
        INK,
    );
    // Rake: shaft with a toothed head.
    pole(
        r,
        seed ^ 2,
        (s * 0.8, s * 0.14),
        (s * 0.74, s * 0.66),
        1.6,
        WOOD,
        4.0,
    );
    rect(
        r,
        (s * 0.74, s * 0.7),
        (s * 0.1, s * 0.025),
        1.5,
        metal(seed ^ 2, IRON),
        3.0,
        2.0,
        INK,
    );
    for k in 0..5 {
        let x = s * (0.66 + 0.04 * k as f32);
        pole(r, seed ^ 3, (x, s * 0.72), (x, s * 0.8), 0.7, IRON, 3.0);
    }
}
