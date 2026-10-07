//! Dungeon and cave props: tombs, coffins, bones, cages, gaol cots, rubble,
//! treasure, stairs up and down, torch sconces and stalagmites.

use super::{
    disc, glow, iron_rect, plank_rect, puck, rect, rod, stone_rect, DARK_WOOD, EAST, IRON,
};
use super::{SOUTH, STONE, WOOD};
use crate::placeholders::material::{cloth, metal, painted, solid, stone, straw, wood, Rng};
use crate::placeholders::paint::{blob, mix, polygon, tone, Rgb};
use crate::placeholders::relief::{bevel, dome, level, Bounds, Relief, INK, NO_INK, SOFT_INK};

/// Old bone.
const BONE: Rgb = [214, 204, 178];
/// Gold coin.
const GOLD: Rgb = [224, 182, 64];

/// Tomb (1 × 2): a stone sarcophagus whose lid carries a carved effigy.
pub fn tomb(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    stone_rect(r, seed, c, (s * 0.4, h / 2.0 - s * 0.06), STONE, 0.0);
    rect(
        r,
        c,
        (s * 0.33, h / 2.0 - s * 0.13),
        2.0,
        stone(seed ^ 1, tone(STONE, 1.12)),
        6.0,
        3.0,
        SOFT_INK,
    );
    let effigy = stone(seed ^ 2, tone(STONE, 1.2));
    disc(r, (c.0, s * 0.42), s * 0.1, effigy, 9.0, 5.0, INK);
    let body = stone(seed ^ 3, tone(STONE, 1.16));
    r.part(
        Bounds::span(c.0, s * 0.5, c.0, h - s * 0.32, s * 0.2),
        polygon(&[
            (c.0 - s * 0.17, s * 0.55),
            (c.0 + s * 0.17, s * 0.55),
            (c.0 + s * 0.11, h - s * 0.3),
            (c.0 - s * 0.11, h - s * 0.3),
        ]),
        body,
        bevel(9.0, 4.0, s * 0.05),
        INK,
    );
    // Folded hands on a sword running down the figure.
    rod(
        r,
        (c.0, s * 0.62),
        (c.0, h - s * 0.4),
        s * 0.025,
        metal(seed ^ 4, [150, 150, 156]),
        13.0,
        SOFT_INK,
    );
}

/// Coffin (1 × 2): a six-sided plank box, the wider end at the top.
pub fn coffin(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    let outline = [
        (cx - s * 0.2, s * 0.12),
        (cx + s * 0.2, s * 0.12),
        (cx + s * 0.34, s * 0.55),
        (cx + s * 0.2, h - s * 0.1),
        (cx - s * 0.2, h - s * 0.1),
        (cx - s * 0.34, s * 0.55),
    ];
    r.part(
        Bounds(0.0, 0.0, w, h),
        polygon(&outline),
        wood(seed, DARK_WOOD, SOUTH, s * 0.16),
        bevel(0.0, 5.0, 4.0),
        INK,
    );
    let inner: Vec<(f32, f32)> = outline
        .iter()
        .map(|&(x, y)| (cx + (x - cx) * 0.8, h * 0.5 + (y - h * 0.5) * 0.9))
        .collect();
    r.part(
        Bounds(0.0, 0.0, w, h),
        polygon(&inner),
        wood(seed ^ 1, WOOD, SOUTH, s * 0.16),
        bevel(5.0, 2.0, 3.0),
        SOFT_INK,
    );
    for y in [s * 0.55, h - s * 0.45] {
        iron_rect(r, seed ^ 2, (cx, y), (s * 0.16, s * 0.02), 7.0);
    }
}

/// A single bone: a shaft with knobbed ends.
fn bone(r: &mut Relief, seed: u64, a: (f32, f32), b: (f32, f32), rad: f32, z0: f32) {
    let col = tone(BONE, 0.92 + 0.12 * Rng::new(seed).f());
    rod(r, a, b, rad, painted(seed, col, 3.0), z0, SOFT_INK);
    for p in [a, b] {
        disc(
            r,
            p,
            rad * 1.6,
            painted(seed ^ 1, col, 3.0),
            z0,
            rad,
            SOFT_INK,
        );
    }
}

/// A skull seen from above: a dome with dark eye sockets at its front.
fn skull(r: &mut Relief, seed: u64, c: (f32, f32), rad: f32, z0: f32) {
    disc(r, c, rad, painted(seed, BONE, 3.0), z0, rad * 0.6, INK);
    for dx in [-0.38, 0.38] {
        disc(
            r,
            (c.0 + dx * rad, c.1 + rad * 0.45),
            rad * 0.24,
            solid([48, 40, 34]),
            z0 + rad * 0.5,
            0.5,
            NO_INK,
        );
    }
}

/// Bone pile: a scatter of long bones and a skull.
pub fn bone_pile(r: &mut Relief, seed: u64, s: f32) {
    let mut rng = Rng::new(seed);
    for i in 0..7 {
        let c = (s * rng.range(0.25, 0.75), s * rng.range(0.25, 0.75));
        let (dx, dy) = rng.dir();
        let l = s * rng.range(0.1, 0.18);
        bone(
            r,
            seed ^ i,
            (c.0 - dx * l, c.1 - dy * l),
            (c.0 + dx * l, c.1 + dy * l),
            s * 0.022,
            i as f32,
        );
    }
    skull(
        r,
        seed ^ 9,
        (s * rng.range(0.4, 0.6), s * rng.range(0.4, 0.6)),
        s * 0.11,
        8.0,
    );
}

/// Skeleton (1 × 2): a figure on its back, skull at the top.
pub fn skeleton(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    let rad = s * 0.02;
    // Spine and ribs.
    bone(r, seed, (cx, s * 0.4), (cx, s * 1.05), rad, 0.0);
    for i in 0..4 {
        let y = s * (0.5 + 0.1 * i as f32);
        let half = s * (0.16 - 0.015 * i as f32);
        bone(
            r,
            seed ^ (i + 1),
            (cx - half, y + s * 0.03),
            (cx, y),
            rad * 0.8,
            1.0,
        );
        bone(
            r,
            seed ^ (i + 9),
            (cx, y),
            (cx + half, y + s * 0.03),
            rad * 0.8,
            1.0,
        );
    }
    // Pelvis, arms and legs.
    disc(
        r,
        (cx, s * 1.1),
        s * 0.07,
        painted(seed ^ 20, BONE, 3.0),
        1.0,
        2.0,
        SOFT_INK,
    );
    for side in [-1.0f32, 1.0] {
        let sh = (cx + side * s * 0.2, s * 0.48);
        let el = (cx + side * s * 0.26, s * 0.8);
        bone(r, seed ^ 30, sh, el, rad, 0.5);
        bone(r, seed ^ 31, el, (cx + side * s * 0.22, s * 1.08), rad, 0.5);
        let hip = (cx + side * s * 0.06, s * 1.14);
        let knee = (cx + side * s * 0.1, s * 1.5);
        bone(r, seed ^ 32, hip, knee, rad * 1.1, 0.5);
        bone(
            r,
            seed ^ 33,
            knee,
            (cx + side * s * 0.12, h - s * 0.12),
            rad,
            0.5,
        );
    }
    skull(r, seed ^ 40, (cx, s * 0.26), s * 0.12, 4.0);
}

/// Cage: an iron-barred box on a plank floor, seen from above.
pub fn cage(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let hw = s * 0.4;
    plank_rect(r, seed, c, (hw, hw), DARK_WOOD, EAST, s * 0.16, 0.0);
    let m = metal(seed ^ 1, IRON);
    let bar = s * 0.018;
    for i in 1..6 {
        let x = c.0 - hw + 2.0 * hw * i as f32 / 6.0;
        rod(r, (x, c.1 - hw), (x, c.1 + hw), bar, &m, 14.0, SOFT_INK);
    }
    for i in 1..4 {
        let y = c.1 - hw + 2.0 * hw * i as f32 / 4.0;
        rod(r, (c.0 - hw, y), (c.0 + hw, y), bar, &m, 16.0, SOFT_INK);
    }
    for (a, b) in [
        ((c.0 - hw, c.1 - hw), (c.0 + hw, c.1 - hw)),
        ((c.0 + hw, c.1 - hw), (c.0 + hw, c.1 + hw)),
        ((c.0 + hw, c.1 + hw), (c.0 - hw, c.1 + hw)),
        ((c.0 - hw, c.1 + hw), (c.0 - hw, c.1 - hw)),
    ] {
        rod(r, a, b, bar * 1.8, &m, 18.0, INK);
    }
    iron_rect(
        r,
        seed ^ 2,
        (c.0 + hw * 0.6, c.1 + hw),
        (s * 0.05, s * 0.04),
        20.0,
    );
}

/// Gaol cot (1 × 2): a plank bunk with a straw pallet and a grey blanket.
pub fn gaol_cot(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    plank_rect(
        r,
        seed,
        (cx, h / 2.0),
        (s * 0.34, h / 2.0 - s * 0.1),
        WOOD,
        SOUTH,
        s * 0.17,
        0.0,
    );
    rect(
        r,
        (cx, h / 2.0),
        (s * 0.28, h / 2.0 - s * 0.18),
        6.0,
        straw(seed ^ 1, [190, 164, 100], SOUTH),
        4.0,
        4.0,
        SOFT_INK,
    );
    rect(
        r,
        (cx, h * 0.62),
        (s * 0.3, h * 0.2),
        4.0,
        cloth(seed ^ 2, [108, 104, 96], EAST),
        7.0,
        3.0,
        INK,
    );
}

/// Rubble pile: broken masonry chunks and grit.
pub fn rubble_pile(r: &mut Relief, seed: u64, s: f32) {
    let mut rng = Rng::new(seed);
    for i in 0..18u64 {
        let ring = if i < 10 { 0.3 } else { 0.16 };
        let (dx, dy) = rng.dir();
        let d = rng.range(0.0, ring);
        let c = (s * (0.5 + dx * d), s * (0.5 + dy * d));
        let rad = s * if i < 10 {
            rng.range(0.05, 0.09)
        } else {
            rng.range(0.07, 0.12)
        };
        let col = tone(STONE, rng.range(0.78, 1.12));
        r.part(
            Bounds::around(c.0, c.1, rad * 1.4),
            blob(seed ^ i, c.0, c.1, rad, 0.35),
            stone(seed ^ (i * 7), col),
            dome(i as f32 * 0.8, rad * 0.8, rad),
            INK,
        );
    }
}

/// Treasure chest: an open iron-bound chest heaped with coins and gems.
pub fn chest_treasure(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.56);
    plank_rect(r, seed, c, (s * 0.36, s * 0.26), WOOD, EAST, s * 0.12, 0.0);
    // The open lid leans back to the north.
    plank_rect(
        r,
        seed ^ 1,
        (c.0, s * 0.2),
        (s * 0.36, s * 0.08),
        DARK_WOOD,
        EAST,
        0.0,
        2.0,
    );
    for x in [c.0 - s * 0.24, c.0 + s * 0.24] {
        iron_rect(r, seed ^ 2, (x, c.1), (s * 0.03, s * 0.26), 6.0);
    }
    let mut rng = Rng::new(seed ^ 3);
    for i in 0..26u64 {
        let p = (
            c.0 + rng.range(-0.28, 0.28) * s,
            c.1 + rng.range(-0.18, 0.18) * s,
        );
        let col = tone(GOLD, rng.range(0.85, 1.15));
        puck(r, p, s * 0.04, metal(seed ^ i, col), 8.0, SOFT_INK);
    }
    for (i, col) in [[200u8, 40, 52], [52, 110, 200], [60, 170, 90]]
        .iter()
        .enumerate()
    {
        let p = (
            c.0 + rng.range(-0.2, 0.2) * s,
            c.1 + rng.range(-0.1, 0.1) * s,
        );
        disc(r, p, s * 0.035, solid(*col), 11.0 + i as f32, 2.0, INK);
    }
}

/// A flight of stone treads across a 1 × 2 footprint, its top end to the
/// north (against the wall it is placed on). Stairs up rise and lighten
/// towards the north; `down` darkens the treads towards the north end and
/// ends there in shadow, so the stairs read as descending from the south.
fn flight(r: &mut Relief, seed: u64, s: f32, down: bool) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    // Side walls of the stairwell.
    for x in [cx - s * 0.42, cx + s * 0.42] {
        stone_rect(
            r,
            seed ^ 7,
            (x, h / 2.0),
            (s * 0.06, h / 2.0 - s * 0.02),
            tone(STONE, 0.9),
            12.0,
        );
    }
    let n = 7;
    let step = (h - s * 0.08) / n as f32;
    for i in 0..n {
        let t = i as f32 / (n - 1) as f32;
        let y = s * 0.04 + step * (i as f32 + 0.5);
        let (col, z0) = if down {
            (
                mix(tone(STONE, 1.08), [36, 32, 30], (1.0 - t) * 0.85),
                1.0 + 9.0 * t,
            )
        } else {
            (mix(tone(STONE, 1.15), tone(STONE, 0.72), t), 10.0 - 9.0 * t)
        };
        rect(
            r,
            (cx, y),
            (s * 0.35, step / 2.0 - 0.8),
            1.5,
            stone(seed ^ i as u64, col),
            z0,
            2.0,
            SOFT_INK,
        );
    }
    if down {
        r.part(
            Bounds(0.0, 0.0, w, s * 0.3),
            crate::placeholders::paint::rbox(cx, s * 0.12, s * 0.35, s * 0.1, 2.0),
            solid([18, 16, 16]),
            level(0.0),
            NO_INK,
        );
    }
}

/// Stairs up (1 × 2): stone treads rising to the north.
pub fn stairs(r: &mut Relief, seed: u64, s: f32) {
    flight(r, seed, s, false);
}

/// Stairs down (1 × 2): stone treads descending north into darkness.
pub fn stairs_down(r: &mut Relief, seed: u64, s: f32) {
    flight(r, seed, s, true);
}

/// Torch sconce: an iron bracket on the north edge holding a burning torch.
pub fn torch_sconce(r: &mut Relief, seed: u64, s: f32) {
    let cx = s / 2.0;
    iron_rect(r, seed, (cx, s * 0.06), (s * 0.12, s * 0.05), 6.0);
    rod(
        r,
        (cx, s * 0.08),
        (cx, s * 0.3),
        s * 0.025,
        metal(seed ^ 1, IRON),
        8.0,
        INK,
    );
    disc(
        r,
        (cx, s * 0.32),
        s * 0.06,
        wood(seed ^ 2, DARK_WOOD, SOUTH, 0.0),
        10.0,
        3.0,
        INK,
    );
    glow(r, seed, (cx, s * 0.33), s * 0.07, 14.0);
}

/// Stalagmite cluster: tapering limestone cones, each lit on its tip.
pub fn stalagmite(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let s = w.min(h);
    let spots = [
        (0.5, 0.52, 0.3),
        (0.28, 0.32, 0.13),
        (0.74, 0.3, 0.11),
        (0.3, 0.76, 0.1),
    ];
    for (i, (x, y, rad)) in spots.into_iter().enumerate() {
        let sd = seed ^ (i as u64 * 131);
        let c = (x * w, y * h);
        let base = mix([142, 134, 118], [176, 168, 150], Rng::new(sd).f());
        let rad = rad * s;
        r.part(
            Bounds::around(c.0, c.1, rad * 1.3),
            blob(sd, c.0, c.1, rad, 0.18),
            stone(sd, base),
            // A cone: height falls linearly from the tip.
            move |px, py, _| {
                let d = ((px - c.0).powi(2) + (py - c.1).powi(2)).sqrt();
                (rad - d).max(0.0) * 1.6
            },
            INK,
        );
        disc(
            r,
            c,
            rad * 0.22,
            stone(sd ^ 1, tone(base, 1.2)),
            rad * 1.6,
            1.0,
            NO_INK,
        );
    }
}
