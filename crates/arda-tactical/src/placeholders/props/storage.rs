//! Containers and storage furniture: barrels, crates, sacks, chests,
//! cupboards, shelves, bookshelves, cask racks, buckets and woodpiles.

use super::{
    disc, iron_rect, plank_rect, pole, puck, rect, ring, BRASS, BURLAP, DARK_WOOD, EAST, IRON,
    PALE_WOOD, SOUTH, WOOD,
};
use crate::noise::{hash2, unit};
use crate::placeholders::material::{cloth, metal, solid, wood, Rng};
use crate::placeholders::paint::{blob, capsule, mix, tone};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

/// Barrel: a staved lid inside an iron hoop, with a bung.
pub fn barrel(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    disc(r, c, s * 0.33, metal(seed, IRON), 0.0, 6.0, INK);
    puck(
        r,
        c,
        s * 0.29,
        wood(seed, WOOD, SOUTH, s * 0.085),
        4.0,
        SOFT_INK,
    );
    ring(r, c, s * 0.27, s * 0.3, metal(seed ^ 1, IRON), 5.0);
    disc(
        r,
        (c.0 + s * 0.1, c.1 - s * 0.06),
        s * 0.035,
        solid(DARK_WOOD),
        6.0,
        1.0,
        SOFT_INK,
    );
}

/// Crate: a planked lid in a raised frame with a diagonal brace and nails.
pub fn crate_box(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let h = s * 0.35;
    plank_rect(r, seed, c, (h, h), PALE_WOOD, EAST, h * 2.0 / 4.0, 0.0);
    for (dx, dy, hw, hh) in [
        (0.0, -h + 3.5, h, 3.5),
        (0.0, h - 3.5, h, 3.5),
        (-h + 3.5, 0.0, 3.5, h),
        (h - 3.5, 0.0, 3.5, h),
    ] {
        rect(
            r,
            (c.0 + dx, c.1 + dy),
            (hw, hh),
            1.0,
            wood(seed ^ 3, WOOD, if hw > hh { EAST } else { SOUTH }, 0.0),
            4.0,
            2.5,
            SOFT_INK,
        );
    }
    let a = (c.0 - h + 6.0, c.1 - h + 6.0);
    let b = (c.0 + h - 6.0, c.1 + h - 6.0);
    r.part(
        Bounds::span(a.0, a.1, b.0, b.1, 4.0),
        capsule(a.0, a.1, b.0, b.1, 3.2),
        wood(seed ^ 4, WOOD, (0.707, 0.707), 0.0),
        bevel(6.0, 2.0, 2.0),
        SOFT_INK,
    );
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let p = (c.0 + dx * (h - 3.5), c.1 + dy * (h - 3.5));
        disc(
            r,
            p,
            1.4,
            solid([60, 58, 56]),
            7.0,
            0.5,
            super::super::relief::NO_INK,
        );
    }
}

/// A burlap sack seen from above, tied at the neck.
pub fn sack(r: &mut Relief, seed: u64, c: (f32, f32), rad: f32, z0: f32) {
    let mut rng = Rng::new(seed);
    let base = tone(BURLAP, rng.range(0.9, 1.08));
    r.part(
        Bounds::around(c.0, c.1, rad * 1.2),
        blob(seed, c.0, c.1, rad, 0.12),
        cloth(seed, base, rng.dir()),
        dome(z0, rad * 0.8, rad),
        INK,
    );
    let (dx, dy) = rng.dir();
    let neck = (c.0 + dx * rad * 0.45, c.1 + dy * rad * 0.45);
    disc(
        r,
        neck,
        rad * 0.22,
        cloth(seed ^ 1, tone(base, 0.85), (dy, dx)),
        z0 + rad * 0.8,
        3.0,
        SOFT_INK,
    );
    disc(
        r,
        neck,
        rad * 0.09,
        solid([120, 96, 60]),
        z0 + rad * 0.8 + 3.0,
        1.0,
        SOFT_INK,
    );
}

/// Sacks: three lumpy burlap bags.
pub fn sacks(r: &mut Relief, seed: u64, s: f32) {
    for (i, (x, y, rad)) in [(0.35, 0.37, 0.2), (0.65, 0.42, 0.19), (0.47, 0.66, 0.2)]
        .into_iter()
        .enumerate()
    {
        sack(r, seed ^ (i as u64 * 77), (x * s, y * s), rad * s, 0.0);
    }
}

/// Chest: a domed dark-wood lid with iron bands and a brass lock.
pub fn chest(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let (hw, hh) = (s * 0.33, s * 0.22);
    let cy = c.1;
    r.part(
        Bounds::around(c.0, c.1, hw),
        crate::placeholders::paint::rbox(c.0, c.1, hw, hh, 3.0),
        wood(seed, DARK_WOOD, EAST, hh * 0.5),
        move |_, y, _| {
            let q = (y - cy) / hh;
            let t = 1.0 - q * q;
            10.0 * t.max(0.0).sqrt()
        },
        INK,
    );
    for bx in [c.0 - hw * 0.62, c.0 + hw * 0.62] {
        iron_rect(r, seed, (bx, c.1), (2.6, hh), 11.0);
    }
    rect(
        r,
        (c.0, c.1 + hh - 4.0),
        (5.0, 4.0),
        1.0,
        metal(seed ^ 9, BRASS),
        12.0,
        1.5,
        SOFT_INK,
    );
}

/// Cupboard: a tall dark cabinet seen from above, doors to the south.
pub fn cupboard(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.36);
    plank_rect(r, seed, c, (s * 0.42, s * 0.22), DARK_WOOD, EAST, 0.0, 0.0);
    rect(
        r,
        c,
        (s * 0.38, s * 0.18),
        2.0,
        wood(seed ^ 1, tone(DARK_WOOD, 1.15), EAST, 0.0),
        5.0,
        2.0,
        SOFT_INK,
    );
    rect(
        r,
        (c.0, c.1 + s * 0.2),
        (s * 0.4, 2.0),
        1.0,
        solid(tone(DARK_WOOD, 0.8)),
        3.0,
        1.0,
        SOFT_INK,
    );
    for dx in [-0.05, 0.05] {
        disc(
            r,
            (c.0 + dx * s, c.1 + s * 0.2),
            1.8,
            metal(seed, BRASS),
            4.0,
            1.0,
            SOFT_INK,
        );
    }
}

/// Shelf: a narrow wall shelf holding jars, a box and a bowl.
pub fn shelf(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s * 0.3);
    plank_rect(r, seed, c, (s * 0.44, s * 0.15), WOOD, EAST, 0.0, 0.0);
    let mut rng = Rng::new(seed);
    let jars = [
        [150, 96, 60],
        [96, 120, 100],
        [196, 180, 150],
        [120, 70, 60],
    ];
    for i in 0..5 {
        let x = s * (0.14 + 0.18 * i as f32);
        let col = rng.pick(&jars).unwrap_or(jars[0]);
        disc(
            r,
            (x, c.1),
            s * rng.range(0.05, 0.07),
            solid(col),
            5.0,
            4.0,
            INK,
        );
    }
}

/// Bookshelf (2 × 1): a long top board with book tops along its front.
pub fn bookshelf(r: &mut Relief, seed: u64, s: f32) {
    let w = r.width as f32;
    let c = (w / 2.0, s * 0.34);
    plank_rect(r, seed, c, (w * 0.46, s * 0.2), DARK_WOOD, EAST, 0.0, 0.0);
    let spines = [
        [120, 40, 36],
        [46, 70, 110],
        [70, 96, 52],
        [150, 120, 60],
        [96, 60, 40],
        [60, 50, 80],
    ];
    let mut x = w * 0.07;
    let mut i = 0;
    while x < w * 0.92 {
        let bw = s * (0.03 + 0.03 * unit(hash2(seed, i, 0)));
        let col = spines[(hash2(seed, i, 1) % 6) as usize];
        rect(
            r,
            (x + bw / 2.0, c.1 + s * 0.06),
            (bw / 2.0 - 0.4, s * 0.11),
            0.5,
            solid(col),
            5.0,
            1.5,
            SOFT_INK,
        );
        x += bw + 0.6;
        i += 1;
    }
}

/// Cask rack (2 × 1): three casks lying on a timber frame.
pub fn cask_rack(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    for y in [h * 0.2, h * 0.8] {
        plank_rect(
            r,
            seed,
            (w / 2.0, y),
            (w * 0.46, s * 0.05),
            DARK_WOOD,
            EAST,
            0.0,
            0.0,
        );
    }
    for i in 0..3 {
        let cx = w * (0.2 + 0.3 * i as f32);
        let (hw, hh) = (s * 0.14, s * 0.4);
        let cxx = cx;
        r.part(
            Bounds::around(cx, h / 2.0, hh),
            crate::placeholders::paint::rbox(cx, h / 2.0, hw, hh, hw * 0.9),
            wood(seed ^ i, WOOD, SOUTH, hw * 0.5),
            move |x, _, _| {
                let q = (x - cxx) / hw;
                let t = 1.0 - q * q;
                4.0 + 12.0 * t.max(0.0).sqrt()
            },
            INK,
        );
        for dy in [-0.26, 0.26] {
            iron_rect(r, seed, (cx, h / 2.0 + dy * s), (hw - 0.5, 2.0), 15.0);
        }
    }
}

/// Bucket: a small staved pail of water with an iron rim and handle.
pub fn bucket(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    puck(r, c, s * 0.17, wood(seed, WOOD, SOUTH, s * 0.04), 0.0, INK);
    disc(
        r,
        c,
        s * 0.13,
        |x, y, _| mix([40, 70, 84], [80, 120, 130], ((x + y) / 60.0).fract()),
        2.0,
        0.5,
        SOFT_INK,
    );
    ring(r, c, s * 0.15, s * 0.175, metal(seed, IRON), 5.0);
    pole(
        r,
        seed,
        (c.0 - s * 0.17, c.1),
        (c.0 + s * 0.17, c.1 - s * 0.03),
        1.2,
        IRON,
        7.0,
    );
}

/// Woodpile: split logs stacked lengthways with their ends showing.
pub fn woodpile(r: &mut Relief, seed: u64, s: f32) {
    let mut rng = Rng::new(seed);
    for row in 0..3 {
        for col in 0..(4 - row % 2) {
            let x = s * (0.22 + 0.19 * col as f32 + 0.095 * (row % 2) as f32);
            let y = s * (0.3 + 0.2 * row as f32);
            let z = 6.0 * (2 - row) as f32;
            let bark = tone([112, 80, 50], rng.range(0.85, 1.1));
            disc(
                r,
                (x, y),
                s * 0.095,
                wood(rng.next_u64(), bark, SOUTH, 0.0),
                z,
                6.0,
                INK,
            );
            let end = tone([200, 164, 112], rng.range(0.9, 1.05));
            let rings = move |px: f32, py: f32, _d: f32| {
                let d = ((px - x) * (px - x) + (py - y) * (py - y)).sqrt();
                tone(end, if (d / 2.2).fract() < 0.3 { 0.86 } else { 1.0 })
            };
            puck(r, (x, y), s * 0.066, rings, z + 5.0, SOFT_INK);
        }
    }
}
