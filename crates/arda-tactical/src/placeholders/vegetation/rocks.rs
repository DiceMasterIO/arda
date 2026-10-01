//! Rocks, logs and stumps.
//!
//! A rock is an irregular convex outline with a noise-wobbled edge, and a
//! height field made of a broad dome, a few soft facets (tilted planes
//! blended by their distance) and fine grain. Relief shading then gives a
//! weathered, natural boulder rather than a smooth pebble or hard
//! origami-like facets.
// Art generation casts bounded pixel coordinates.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use crate::noise::{fbm, value};
use crate::placeholders::material::{drift, heading, painted, stone, wood, Rng};
use crate::placeholders::paint::{capsule, circle, mix, polygon, taper, tone, Rgb};
use crate::placeholders::relief::{bevel, dome, Bounds, Relief, INK, SOFT_INK};

/// A natural rock centred at `c` with radius `rad`.
pub fn rock(r: &mut Relief, seed: u64, c: (f32, f32), rad: f32, base: Rgb, z0: f32, moss: bool) {
    let mut rng = Rng::new(seed);
    let n = 7 + (rng.next_u64() % 3) as usize;
    let spin = rng.f();
    let squash = rng.range(0.7, 1.0);
    let pts: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let (dx, dy) = heading(spin + (i as f32 + rng.range(-0.3, 0.3)) / n as f32);
            let rr = rad * rng.range(0.78, 1.0);
            (c.0 + dx * rr, c.1 + dy * rr * squash)
        })
        .collect();
    let outline = polygon(&pts);
    let wob = seed ^ 0x3B;
    let sdf = move |x: f32, y: f32| {
        outline(x, y) + rad * 0.12 * (value(wob, x / (rad * 0.35), y / (rad * 0.35), None) - 0.5)
    };
    // Soft facets: three tilted planes, blended smoothly.
    let facets: [(f32, f32); 3] = [0, 1, 2].map(|_| (rng.range(-0.5, 0.5), rng.range(-0.5, 0.5)));
    let peak = (
        c.0 + rng.range(-0.2, 0.2) * rad,
        c.1 + rng.range(-0.2, 0.2) * rad,
    );
    let grain = seed ^ 0x3C;
    let height = move |x: f32, y: f32, d: f32| {
        let t = (-d / (rad * 0.6)).clamp(0.0, 1.0);
        let dome_h = rad * 0.5 * (1.0 - (1.0 - t) * (1.0 - t));
        let (dx, dy) = ((x - peak.0) / rad, (y - peak.1) / rad);
        let mut f = 0.0;
        for (i, (a, b)) in facets.iter().enumerate() {
            let w = 1.0 / (1.0 + 4.0 * ((dx - a) * (dx - a) + (dy - b) * (dy - b)));
            f += w * (a * dx + b * dy) * (1.0 + i as f32 * 0.2);
        }
        z0 + dome_h * (1.0 - 0.25 * f) + 2.0 * fbm(grain, x / 3.0, y / 3.0, 2, None)
    };
    let rock_col = stone(seed, base);
    let mossy = seed ^ 0x3D;
    let mat = move |x: f32, y: f32, d: f32| {
        let c = rock_col(x, y, d);
        let top = fbm(mossy, x / 7.0, y / 7.0, 2, None);
        if moss && top > 0.56 && d < -3.0 {
            mix(c, [86, 110, 50], ((top - 0.56) * 5.0).min(0.8))
        } else {
            c
        }
    };
    r.part(Bounds::around(c.0, c.1, rad * 1.1), sdf, mat, height, INK);
    // A crack or two, lit on their north-west lip.
    for _ in 0..2 {
        let (dx, dy) = rng.dir();
        let a = (c.0 + dx * rad * 0.1, c.1 + dy * rad * 0.1);
        let b = (c.0 + dx * rad * 0.6, c.1 + dy * rad * 0.6 * squash);
        r.part(
            Bounds::span(a.0, a.1, b.0, b.1, 2.0),
            taper(a, b, 1.1, 0.3),
            move |_, _, _| tone(base, 0.45),
            move |x, y, d| height(x, y, d.min(-4.0)) - 1.5,
            crate::placeholders::relief::NO_INK,
        );
    }
}

/// Boulders: `count` natural rocks.
pub fn boulders(r: &mut Relief, seed: u64, count: u32) {
    let (w, h) = (r.width as f32, r.height as f32);
    let spots: &[(f32, f32, f32)] = if count == 1 {
        &[(0.5, 0.5, 0.36)]
    } else {
        &[(0.36, 0.4, 0.2), (0.66, 0.46, 0.16), (0.46, 0.7, 0.14)]
    };
    for (i, (x, y, rad)) in spots.iter().enumerate() {
        let sd = seed ^ (i as u64 * 31);
        let base = mix(
            [120, 116, 108],
            [158, 152, 140],
            crate::noise::unit(crate::noise::hash2(sd, 0, 0)),
        );
        rock(r, sd, (x * w, y * h), rad * w.min(h), base, 0.0, false);
    }
}

/// A large rock outcrop: one big stone with a couple of fallen pieces.
pub fn rock_large(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    for _ in 0..3 {
        let (dx, dy) = rng.dir();
        let p = (w * 0.5 + dx * w * 0.33, h * 0.5 + dy * h * 0.33);
        rock(
            r,
            rng.next_u64(),
            p,
            w * rng.range(0.07, 0.11),
            [136, 130, 120],
            0.0,
            false,
        );
    }
    rock(
        r,
        seed,
        (w * 0.5, h * 0.5),
        w * 0.34,
        [140, 134, 122],
        0.0,
        true,
    );
}

/// A scree patch: many loose stones of mixed sizes.
pub fn scree_patch(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    for i in 0..44 {
        let (dx, dy) = rng.dir();
        let off = w * 0.4 * rng.f().sqrt();
        let p = (w * 0.5 + dx * off, h * 0.5 + dy * off * 0.8);
        let rad = w * rng.range(0.018, 0.05) * if i < 8 { 1.6 } else { 1.0 };
        let base = tone(drift([138, 132, 120], rng.f()), rng.range(0.8, 1.12));
        rock(r, rng.next_u64(), p, rad, base, 0.0, false);
    }
}

/// A fallen log (2 × 1) with bark, moss, a broken branch and its end grain.
pub fn fallen_log(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let (a, b) = ((w * 0.1, h * 0.52), (w * 0.88, h * 0.46));
    let rad = h * 0.2;
    let bark = wood(seed, [96, 70, 46], (1.0, -0.077), 0.0);
    let moss_seed = seed ^ 0x55;
    let mat = move |x: f32, y: f32, d: f32| {
        let c = bark(x, y, d);
        let m = fbm(moss_seed, x / 9.0, y / 9.0, 2, None);
        if m > 0.58 {
            mix(c, [78, 104, 44], ((m - 0.58) * 4.0).min(0.85))
        } else {
            c
        }
    };
    let (ax, ay, bx, by) = (a.0, a.1, b.0, b.1);
    let axis = move |x: f32, y: f32| {
        let (vx, vy) = (bx - ax, by - ay);
        let t = (((x - ax) * vx + (y - ay) * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
        let (dx, dy) = (x - ax - vx * t, y - ay - vy * t);
        (dx * dx + dy * dy).sqrt()
    };
    r.part(
        Bounds::span(a.0, a.1, b.0, b.1, rad),
        capsule(a.0, a.1, b.0, b.1, rad),
        mat,
        move |x, y, _| {
            let q = (axis(x, y) / rad).min(1.0);
            18.0 * (1.0 - q * q).max(0.0).sqrt()
        },
        INK,
    );
    let stub = (w * 0.45, h * 0.49);
    r.part(
        Bounds::span(stub.0, stub.1, stub.0 + w * 0.08, h * 0.12, 4.0),
        taper(stub, (stub.0 + w * 0.08, h * 0.12), 4.0, 2.0),
        wood(seed ^ 1, [90, 66, 44], (0.3, -0.95), 0.0),
        dome(16.0, 3.0, 4.0),
        INK,
    );
    let end = (b.0, b.1);
    let rings = move |x: f32, y: f32, _d: f32| {
        let d = ((x - end.0) * (x - end.0) + (y - end.1) * (y - end.1)).sqrt();
        tone(
            [196, 160, 110],
            if (d / 2.4).fract() < 0.3 { 0.84 } else { 1.0 },
        )
    };
    r.part(
        Bounds::around(end.0, end.1, rad),
        crate::placeholders::paint::ellipse(end.0, end.1, rad * 0.35, rad * 0.92),
        rings,
        bevel(14.0, 4.0, 3.0),
        SOFT_INK,
    );
}

/// A tree stump: end grain inside a bark ring, with roots.
pub fn stump(r: &mut Relief, seed: u64) {
    let s = r.width as f32;
    let c = (s / 2.0, s / 2.0);
    let mut rng = Rng::new(seed);
    for i in 0..5 {
        let (dx, dy) = heading(i as f32 / 5.0 + rng.range(-0.06, 0.06));
        let tip = (c.0 + dx * s * 0.4, c.1 + dy * s * 0.4);
        r.part(
            Bounds::span(c.0, c.1, tip.0, tip.1, 8.0),
            taper(c, tip, s * 0.07, 1.5),
            wood(rng.next_u64(), [92, 68, 46], (dx, dy), 0.0),
            dome(0.0, 5.0, s * 0.07),
            INK,
        );
    }
    r.part(
        Bounds::around(c.0, c.1, s * 0.25),
        circle(c.0, c.1, s * 0.22),
        painted(seed, [86, 62, 42], 3.0),
        bevel(6.0, 6.0, 4.0),
        INK,
    );
    let rings = move |x: f32, y: f32, _d: f32| {
        let d = ((x - c.0) * (x - c.0) + (y - c.1) * (y - c.1)).sqrt();
        tone(
            [192, 156, 108],
            if (d / 3.0).fract() < 0.28 { 0.82 } else { 1.0 },
        )
    };
    r.part(
        Bounds::around(c.0, c.1, s * 0.2),
        circle(c.0, c.1, s * 0.17),
        rings,
        bevel(12.0, 1.5, 3.0),
        SOFT_INK,
    );
}

/// A rock outcrop (3 × 3): bedrock breaking the turf in a few tilted,
/// lichen-spotted slabs with fallen blocks and grit at their foot.
pub fn outcrop(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let mut rng = Rng::new(seed);
    // Grit and small fallen pieces around the base.
    for _ in 0..16 {
        let (dx, dy) = rng.dir();
        let off = w * rng.range(0.3, 0.45);
        let p = (w * 0.5 + dx * off, h * 0.5 + dy * off * 0.85);
        let base = tone(drift([136, 130, 118], rng.f()), rng.range(0.82, 1.08));
        rock(
            r,
            rng.next_u64(),
            p,
            w * rng.range(0.015, 0.035),
            base,
            0.0,
            false,
        );
    }
    // Slabs: the bedrock's strike runs one way, so they line up in two
    // offset rows, overlapping into one mass.
    let (ax, ay) = heading(rng.f());
    for i in 0..7 {
        let t = (i % 4) as f32 / 3.0 - 0.5;
        let row = if i < 4 { -0.09 } else { 0.1 };
        let side = row + rng.range(-0.04, 0.04);
        let along = t * if i < 4 { 0.5 } else { 0.36 };
        let p = (
            w * (0.5 + ax * along - ay * side),
            h * (0.5 + ay * along + ax * side),
        );
        let rad = w * rng.range(0.15, 0.2) * (1.0 - 0.3 * t.abs());
        let base = tone(drift([142, 136, 122], rng.f()), rng.range(0.88, 1.06));
        let z = 4.0 + 8.0 * (1.0 - 2.0 * t.abs()) + if i < 4 { 0.0 } else { 6.0 };
        rock(r, rng.next_u64(), p, rad, base, z, true);
    }
}
