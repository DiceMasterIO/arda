//! Outdoor and street props: carts, rowboats, market stalls, tents,
//! fences, docks, bridges, cranes, wells, signposts, graves, altars and
//! statues.

use super::storage::sack;
use super::{
    disc, iron_rect, plank_rect, pole, rect, ring, CANVAS, DARK_WOOD, EAST, PALE_WOOD, SOUTH,
    STONE, WOOD,
};
use crate::placeholders::material::{cloth, solid, stone, wood, Rng};
use crate::placeholders::paint::{circle, mix, rbox, tone};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

/// A spoked cart wheel seen edge-on from above: a narrow rim with a hub.
pub fn wheel(r: &mut Relief, seed: u64, c: (f32, f32), len: f32, width: f32, z0: f32) {
    rect(
        r,
        c,
        (width, len),
        width * 0.9,
        wood(seed, DARK_WOOD, SOUTH, 0.0),
        z0,
        4.0,
        INK,
    );
    iron_rect(r, seed, c, (width * 1.3, len * 0.18), z0 + 4.0);
}

/// Cart (1 × 2): a planked bed on two wheels with shafts to the north.
pub fn cart(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    for x in [cx - s * 0.4, cx + s * 0.4] {
        wheel(r, seed, (x, h * 0.62), s * 0.3, s * 0.06, 0.0);
    }
    for x in [cx - s * 0.18, cx + s * 0.18] {
        pole(r, seed, (x, s * 0.06), (x, h * 0.4), s * 0.035, WOOD, 4.0);
    }
    plank_rect(
        r,
        seed,
        (cx, h * 0.6),
        (s * 0.34, s * 0.6),
        PALE_WOOD,
        SOUTH,
        s * 0.11,
        6.0,
    );
    for (dx, hw) in [(-0.32, 0.03), (0.32, 0.03)] {
        plank_rect(
            r,
            seed ^ 2,
            (cx + dx * s, h * 0.6),
            (s * hw, s * 0.6),
            WOOD,
            SOUTH,
            0.0,
            12.0,
        );
    }
    sack(r, seed ^ 9, (cx, h * 0.52), s * 0.17, 10.0);
    sack(r, seed ^ 8, (cx + s * 0.04, h * 0.76), s * 0.16, 10.0);
}

/// Rowboat (1 × 2): a pointed hull with two thwarts; bow to the north.
pub fn rowboat(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let cx = w / 2.0;
    let rad = h * 0.62;
    let off = rad - s * 0.36;
    let hull = move |x: f32, y: f32| {
        circle(cx - off, h / 2.0, rad)(x, y)
            .max(circle(cx + off, h / 2.0, rad)(x, y))
            .max((y - h + 6.0).max(6.0 - y))
    };
    r.part(
        r.all(),
        hull,
        wood(seed, WOOD, SOUTH, s * 0.07),
        bevel(0.0, 8.0, 5.0),
        INK,
    );
    let inner = move |x: f32, y: f32| hull(x, y) + 6.0;
    r.part(
        r.all(),
        inner,
        wood(seed ^ 2, DARK_WOOD, SOUTH, s * 0.07),
        bevel(4.0, -3.0, 4.0),
        SOFT_INK,
    );
    for ty in [0.4, 0.64] {
        plank_rect(
            r,
            seed ^ 3,
            (cx, h * ty),
            (s * 0.3, s * 0.05),
            PALE_WOOD,
            EAST,
            0.0,
            8.0,
        );
    }
    pole(
        r,
        seed ^ 4,
        (cx - s * 0.1, h * 0.3),
        (cx + s * 0.15, h * 0.85),
        2.0,
        PALE_WOOD,
        10.0,
    );
}

/// Market stall (2 × 2): a counter of goods under a striped awning.
pub fn market_stall(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    plank_rect(
        r,
        seed,
        (w / 2.0, h * 0.82),
        (w * 0.44, s * 0.16),
        WOOD,
        EAST,
        s * 0.1,
        0.0,
    );
    let goods = [
        [196, 70, 50],
        [220, 180, 70],
        [120, 150, 60],
        [230, 140, 50],
    ];
    let mut rng = Rng::new(seed);
    for i in 0..9 {
        let p = (
            w * (0.12 + 0.095 * i as f32),
            h * 0.84 + rng.range(-4.0, 4.0),
        );
        let col = rng.pick(&goods).unwrap_or(goods[0]);
        disc(r, p, s * 0.045, solid(col), 5.0, 3.0, INK);
    }
    for (x, y) in [(0.1, 0.1), (0.9, 0.1)] {
        disc(
            r,
            (w * x, h * y),
            s * 0.05,
            wood(seed, DARK_WOOD, EAST, 0.0),
            0.0,
            4.0,
            INK,
        );
    }
    let awning = rbox(w / 2.0, h * 0.38, w * 0.44, h * 0.3, 3.0);
    r.part(
        r.all(),
        awning,
        move |x, y, d| {
            let stripe = ((x / (s * 0.2)).floor() as i64) % 2 == 0;
            let base = if stripe {
                [176, 56, 46]
            } else {
                [226, 216, 190]
            };
            cloth(seed ^ 1, base, SOUTH)(x, y, d)
        },
        move |_, y, _| {
            // A ridge along the awning's middle, sloping to front and back.
            let q = (y - h * 0.38) / (h * 0.3);
            26.0 - 12.0 * q.abs()
        },
        INK,
    );
}

/// Tent (2 × 2): canvas pitched along its long axis, with guy ropes.
pub fn tent(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let (cx, cy) = (w / 2.0, h / 2.0);
    for (x, y) in [(0.08, 0.1), (0.92, 0.1), (0.08, 0.9), (0.92, 0.9)] {
        pole(
            r,
            seed,
            (cx + (w * x - cx) * 0.6, cy + (h * y - cy) * 0.6),
            (w * x, h * y),
            1.0,
            [170, 150, 110],
            0.0,
        );
        disc(r, (w * x, h * y), 3.0, solid(DARK_WOOD), 0.0, 2.0, INK);
    }
    let hw = w * 0.36;
    r.part(
        r.all(),
        rbox(cx, cy, hw, h * 0.42, 3.0),
        cloth(seed, CANVAS, SOUTH),
        move |x, _, _| 30.0 * (1.0 - ((x - cx) / hw).abs()),
        INK,
    );
    rect(
        r,
        (cx, cy),
        (2.0, h * 0.42),
        1.0,
        solid(tone(CANVAS, 0.8)),
        31.0,
        0.0,
        SOFT_INK,
    );
    let _ = s;
}

/// Fence run: two rails along the square's centre line on stout posts.
pub fn fence(r: &mut Relief, seed: u64, s: f32) {
    let c = s / 2.0;
    for dy in [-s * 0.045, s * 0.045] {
        plank_rect(
            r,
            seed,
            (c, c + dy),
            (s / 2.0, s * 0.022),
            WOOD,
            EAST,
            0.0,
            6.0,
        );
    }
    for x in [s * 0.08, s * 0.92] {
        rect(
            r,
            (x, c),
            (s * 0.05, s * 0.07),
            2.0,
            wood(seed ^ 1, DARK_WOOD, SOUTH, 0.0),
            8.0,
            4.0,
            INK,
        );
    }
}

/// Dock planks: boards across the square with gaps between them.
pub fn dock(r: &mut Relief, seed: u64, s: f32) {
    let board = s / 6.0;
    let mut rng = Rng::new(seed);
    for i in 0..6 {
        let y = board * (i as f32 + 0.5);
        let col = tone(WOOD, rng.range(0.82, 1.08));
        plank_rect(
            r,
            rng.next_u64(),
            (s / 2.0, y),
            (s / 2.0 - 0.5, board / 2.0 - 1.5),
            col,
            EAST,
            0.0,
            0.0,
        );
        for x in [s * 0.1, s * 0.9] {
            disc(r, (x, y), 1.3, solid([50, 48, 46]), 4.0, 0.5, SOFT_INK);
        }
    }
}

/// Bridge deck (1 × 2 across, two squares wide): planks with rail beams.
pub fn bridge(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let board = w / 8.0;
    let mut rng = Rng::new(seed);
    for i in 0..8 {
        let x = board * (i as f32 + 0.5);
        let col = tone(PALE_WOOD, rng.range(0.8, 1.05));
        plank_rect(
            r,
            rng.next_u64(),
            (x, h / 2.0),
            (board / 2.0 - 1.0, h / 2.0 - 0.5),
            col,
            SOUTH,
            0.0,
            0.0,
        );
    }
    for y in [s * 0.07, h - s * 0.07] {
        plank_rect(
            r,
            seed ^ 0xB,
            (w / 2.0, y),
            (w / 2.0 - 0.5, s * 0.06),
            DARK_WOOD,
            EAST,
            0.0,
            8.0,
        );
        for x in [s * 0.08, w - s * 0.08] {
            disc(
                r,
                (x, y),
                s * 0.07,
                wood(seed, DARK_WOOD, EAST, 0.0),
                12.0,
                5.0,
                INK,
            );
        }
    }
}

/// Crane (2 × 2): a planked base, a mast and a jib with a cargo net.
pub fn crane(r: &mut Relief, seed: u64, s: f32) {
    let (w, h) = (r.width as f32, r.height as f32);
    plank_rect(
        r,
        seed,
        (w * 0.3, h * 0.7),
        (s * 0.42, s * 0.42),
        WOOD,
        EAST,
        s * 0.14,
        0.0,
    );
    disc(
        r,
        (w * 0.3, h * 0.7),
        s * 0.14,
        wood(seed ^ 3, DARK_WOOD, EAST, 0.0),
        6.0,
        6.0,
        INK,
    );
    pole(
        r,
        seed ^ 1,
        (w * 0.3, h * 0.7),
        (w * 0.82, h * 0.2),
        s * 0.06,
        PALE_WOOD,
        30.0,
    );
    pole(
        r,
        seed ^ 4,
        (w * 0.82, h * 0.2),
        (w * 0.82, h * 0.3),
        1.2,
        [176, 156, 108],
        28.0,
    );
    let net = (w * 0.82, h * 0.34);
    r.part(
        Bounds::around(net.0, net.1, s * 0.15),
        circle(net.0, net.1, s * 0.14),
        move |x, y, _| {
            if ((x + y) % 7.0) < 1.5 || ((x - y + 200.0) % 7.0) < 1.5 {
                [150, 130, 90]
            } else {
                super::BURLAP
            }
        },
        dome(10.0, 10.0, s * 0.14),
        INK,
    );
}

/// Well: a stone ring around dark water, with a roller beam.
pub fn well(r: &mut Relief, seed: u64, s: f32) {
    let c = (s / 2.0, s / 2.0);
    ring(r, c, s * 0.22, s * 0.37, stone(seed, STONE), 6.0);
    disc(
        r,
        c,
        s * 0.22,
        |x, y, _| mix([18, 34, 44], [34, 60, 72], ((x + y) / 400.0).min(1.0)),
        0.0,
        0.5,
        SOFT_INK,
    );
    plank_rect(r, seed ^ 1, c, (s * 0.45, s * 0.04), WOOD, EAST, 0.0, 16.0);
    rect(
        r,
        (c.0, c.1 + s * 0.08),
        (s * 0.06, s * 0.06),
        2.0,
        wood(seed ^ 2, DARK_WOOD, EAST, 0.0),
        10.0,
        4.0,
        INK,
    );
    for x in [c.0 - s * 0.42, c.0 + s * 0.42] {
        disc(
            r,
            (x, c.1),
            s * 0.05,
            wood(seed ^ 3, DARK_WOOD, EAST, 0.0),
            14.0,
            4.0,
            INK,
        );
    }
}
