//! Workshop props: anvils, workbenches, looms, grindstones, weapon racks
//! and armour stands.

use super::{
    disc, iron_rect, plank_rect, pole, puck, rect, rod, DARK_WOOD, EAST, IRON, PALE_WOOD, SOUTH,
    WOOD,
};
use crate::placeholders::material::{cloth, metal, solid, stone, wood};
use crate::placeholders::paint::{capsule, rbox, union};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

const STEEL: [u8; 3] = [150, 154, 160];

/// Anvil: a horned iron anvil on a wooden stump.
pub fn anvil(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    let rings = move |x: f32, y: f32, _d: f32| {
        let d = ((x - c.0) * (x - c.0) + (y - c.1) * (y - c.1)).sqrt();
        crate::placeholders::paint::tone(
            [150, 112, 70],
            if (d / 3.0).fract() < 0.3 { 0.85 } else { 1.0 },
        )
    };
    puck(r, c, s * 0.3, rings, 0.0, INK);
    let body = rbox(c.0 + s * 0.06, c.1, s * 0.17, s * 0.1, 2.0);
    let horn = capsule(c.0 - s * 0.1, c.1, c.0 - s * 0.36, c.1, s * 0.035);
    let neck = rbox(c.0 - s * 0.12, c.1, s * 0.06, s * 0.065, 3.0);
    let shape = union(union(body, horn), neck);
    r.part(
        Bounds::around(c.0, c.1, s * 0.4),
        shape,
        metal(seed, [84, 86, 92]),
        bevel(8.0, 5.0, 4.0),
        INK,
    );
    iron_rect(r, seed ^ 1, (c.0 + s * 0.14, c.1), (2.5, 2.5), 13.0);
}

/// Workbench (2 × 1): a thick top with a vice, a hammer, a saw and shavings.
pub fn workbench(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    plank_rect(
        r,
        seed,
        c,
        (w / 2.0 - s * 0.06, s * 0.3),
        PALE_WOOD,
        EAST,
        s * 0.15,
        0.0,
    );
    rect(
        r,
        (w - s * 0.2, c.1 + s * 0.24),
        (s * 0.1, s * 0.07),
        2.0,
        metal(seed, IRON),
        5.0,
        3.0,
        INK,
    );
    pole(
        r,
        seed ^ 1,
        (s * 0.35, c.1 - s * 0.1),
        (s * 0.35, c.1 + s * 0.16),
        2.4,
        WOOD,
        5.0,
    );
    rect(
        r,
        (s * 0.35, c.1 - s * 0.13),
        (s * 0.07, s * 0.035),
        1.0,
        metal(seed ^ 2, IRON),
        7.0,
        2.0,
        INK,
    );
    r.part(
        Bounds::around(s * 1.0, c.1, s * 0.3),
        rbox(s * 1.0, c.1 - s * 0.02, s * 0.22, s * 0.06, 1.0),
        metal(seed ^ 3, STEEL),
        bevel(5.0, 1.0, 2.0),
        INK,
    );
    rect(
        r,
        (s * 1.28, c.1 - s * 0.02),
        (s * 0.07, s * 0.04),
        2.0,
        wood(seed, DARK_WOOD, EAST, 0.0),
        6.0,
        2.0,
        INK,
    );
    for i in 0..6 {
        let x = s * (0.6 + 0.07 * i as f32);
        disc(
            r,
            (x, c.1 + s * 0.17),
            2.2,
            solid([214, 186, 130]),
            5.0,
            1.0,
            SOFT_INK,
        );
    }
}

/// Loom (2 × 1): a frame with warp threads and a band of woven cloth.
pub fn loom(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    for y in [h * 0.18, h * 0.82] {
        plank_rect(
            r,
            seed,
            (w / 2.0, y),
            (w * 0.46, s * 0.07),
            WOOD,
            EAST,
            0.0,
            10.0,
        );
    }
    for x in [w * 0.06, w * 0.94] {
        plank_rect(
            r,
            seed ^ 1,
            (x, h / 2.0),
            (s * 0.06, h * 0.4),
            DARK_WOOD,
            SOUTH,
            0.0,
            12.0,
        );
    }
    rect(
        r,
        (w / 2.0, h * 0.64),
        (w * 0.4, h * 0.14),
        1.0,
        cloth(seed, [150, 60, 52], EAST),
        4.0,
        1.0,
        SOFT_INK,
    );
    let n = 22;
    for i in 0..n {
        let x = w * 0.1 + (w * 0.8) * i as f32 / (n - 1) as f32;
        rod(
            r,
            (x, h * 0.2),
            (x, h * 0.5),
            1.0,
            solid([226, 216, 190]),
            6.0,
            SOFT_INK,
        );
    }
    plank_rect(
        r,
        seed ^ 2,
        (w / 2.0, h * 0.5),
        (w * 0.42, s * 0.03),
        PALE_WOOD,
        EAST,
        0.0,
        8.0,
    );
}

/// Grindstone: a stone wheel standing in a wooden frame with a crank.
pub fn grindstone(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    for dy in [-0.16, 0.16] {
        plank_rect(
            r,
            seed,
            (c.0, c.1 + dy * s),
            (s * 0.34, s * 0.045),
            WOOD,
            EAST,
            0.0,
            0.0,
        );
    }
    let edge = stone(seed ^ 1, [140, 134, 124]);
    r.part(
        Bounds::around(c.0, c.1, s * 0.3),
        rbox(c.0, c.1, s * 0.28, s * 0.07, s * 0.05),
        edge,
        move |x, _, _| {
            let q = (x - c.0) / (s * 0.28);
            4.0 + 16.0 * (1.0 - q * q).max(0.0).sqrt()
        },
        INK,
    );
    pole(
        r,
        seed ^ 2,
        (c.0, c.1 - s * 0.2),
        (c.0, c.1 + s * 0.3),
        2.2,
        IRON,
        16.0,
    );
    pole(
        r,
        seed ^ 3,
        (c.0, c.1 + s * 0.3),
        (c.0 + s * 0.12, c.1 + s * 0.3),
        2.4,
        DARK_WOOD,
        16.0,
    );
}

/// Weapon rack (2 × 1): a rail holding spears, swords and an axe.
pub fn weapon_rack(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    plank_rect(
        r,
        seed,
        (w / 2.0, h * 0.34),
        (w * 0.46, s * 0.07),
        DARK_WOOD,
        EAST,
        0.0,
        12.0,
    );
    plank_rect(
        r,
        seed ^ 1,
        (w / 2.0, h * 0.72),
        (w * 0.46, s * 0.05),
        DARK_WOOD,
        EAST,
        0.0,
        4.0,
    );
    for i in 0..3 {
        let x = s * (0.3 + 0.28 * i as f32);
        pole(
            r,
            seed ^ i,
            (x, h * 0.08),
            (x, h * 0.9),
            2.0,
            [150, 110, 70],
            8.0,
        );
        r.part(
            Bounds::around(x, h * 0.1, s * 0.08),
            crate::placeholders::paint::obox(x, h * 0.1, s * 0.03, s * 0.07, 1.0, (0.707, 0.707)),
            metal(seed ^ 7, STEEL),
            bevel(10.0, 2.0, 2.0),
            INK,
        );
    }
    for i in 0..2 {
        let x = s * (1.2 + 0.3 * i as f32);
        rod(
            r,
            (x, h * 0.12),
            (x, h * 0.66),
            2.8,
            metal(seed ^ 9, STEEL),
            10.0,
            INK,
        );
        rect(
            r,
            (x, h * 0.7),
            (s * 0.08, 2.0),
            1.0,
            metal(seed ^ 10, [110, 90, 60]),
            12.0,
            1.0,
            INK,
        );
        pole(
            r,
            seed ^ 11,
            (x, h * 0.72),
            (x, h * 0.86),
            2.0,
            DARK_WOOD,
            12.0,
        );
    }
}

/// Armour stand: a cross-shaped stand wearing a breastplate and helm.
pub fn armour_stand(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    puck(
        r,
        (c.0, c.1 + s * 0.02),
        s * 0.3,
        wood(seed, DARK_WOOD, EAST, s * 0.1),
        0.0,
        INK,
    );
    pole(
        r,
        seed ^ 1,
        (c.0 - s * 0.3, c.1),
        (c.0 + s * 0.3, c.1),
        2.6,
        WOOD,
        6.0,
    );
    for dx in [-0.2, 0.2] {
        disc(
            r,
            (c.0 + dx * s, c.1),
            s * 0.1,
            metal(seed ^ 2, STEEL),
            10.0,
            5.0,
            INK,
        );
    }
    disc(
        r,
        (c.0, c.1 + s * 0.05),
        s * 0.16,
        metal(seed ^ 3, STEEL),
        10.0,
        9.0,
        INK,
    );
    disc(
        r,
        (c.0, c.1 - s * 0.04),
        s * 0.09,
        metal(seed ^ 4, [120, 124, 130]),
        20.0,
        8.0,
        INK,
    );
    iron_rect(r, seed, (c.0, c.1 - s * 0.01), (s * 0.04, 1.2), 28.0);
    let _ = dome;
}
