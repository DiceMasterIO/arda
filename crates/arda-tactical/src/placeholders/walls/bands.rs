//! Wall bodies by style, painted along a local frame: `u` runs along the
//! wall and `v` across it, so the same painter draws east–west runs and the
//! north–south arms of joints.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::Kit;
use crate::noise::{fbm, hash2, mix as mix64, unit};
use crate::placeholders::material::{drift, leafy, stone, wood, Rng};
use crate::placeholders::paint::{blob, circle, rbox, tone, Rgb};
use crate::placeholders::relief::{bevel, bumpy, dome, Bounds, Relief, INK, SOFT_INK};
use crate::placeholders::vegetation::rocks::rock;

/// Maps local `(u, v)` to image `(x, y)` and back.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// Image centre (both axes; pieces are square).
    pub c: f32,
    /// Whether `u` runs along y.
    pub vertical: bool,
}

impl Frame {
    /// Image position of a local point.
    #[must_use]
    pub fn xy(self, u: f32, v: f32) -> (f32, f32) {
        if self.vertical {
            (self.c + v, u)
        } else {
            (u, self.c + v)
        }
    }

    /// Local coordinates of an image point.
    #[must_use]
    pub fn uv(self, x: f32, y: f32) -> (f32, f32) {
        if self.vertical {
            (y, x - self.c)
        } else {
            (x, y - self.c)
        }
    }

    /// The image-space along-direction as a unit vector.
    #[must_use]
    pub fn along(self) -> (f32, f32) {
        if self.vertical {
            (0.0, 1.0)
        } else {
            (1.0, 0.0)
        }
    }

    /// Bounds of a local box.
    #[must_use]
    pub fn bounds(self, u0: f32, u1: f32, hv: f32) -> Bounds {
        let (a, b) = (self.xy(u0, -hv), self.xy(u1, hv));
        Bounds::span(a.0, a.1, b.0, b.1, 2.0)
    }
}

/// A bevelled block in local coordinates.
#[allow(clippy::too_many_arguments)]
pub fn block(
    r: &mut Relief,
    f: Frame,
    (um, vm): (f32, f32),
    (hu, hv): (f32, f32),
    round: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
    rise: f32,
) {
    let local = rbox(um, vm, hu, hv, round);
    let bev = hu.min(hv).clamp(1.5, 5.0);
    r.part(
        f.bounds(um - hu, um + hu, vm.abs() + hv),
        move |x, y| {
            let (u, v) = f.uv(x, y);
            local(u, v)
        },
        mat,
        bevel(z0, rise, bev),
        INK,
    );
}

/// Block cuts from `u0` to `u1` with lengths in `lo..hi`, from a seed.
fn cuts(seed: u64, u0: f32, u1: f32, lo: f32, hi: f32, phase: f32) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    let mut u = u0 - phase * lo;
    let mut i = 0;
    while u < u1 {
        let l = lo + (hi - lo) * unit(hash2(seed, i, 3));
        out.push((u.max(u0), (u + l).min(u1)));
        u += l;
        i += 1;
    }
    out
}

/// Paints the wall body of `kit` from `u0` to `u1`.
pub fn body(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, u0: f32, u1: f32, scale: f32) {
    let s = f.c * 2.0;
    let t = kit.thickness * s * scale;
    match kit.name {
        "stone" => ashlar(r, kit, seed, f, (u0, u1), t, s),
        "timber" => beams(r, kit, seed, f, (u0, u1), t),
        "wattle" => wattle(r, kit, seed, f, (u0, u1), t, s),
        "palisade" => palisade(r, kit, seed, f, (u0, u1), t),
        "hedge" => hedge(r, kit, seed, f, (u0, u1), t),
        "drystone" => drystone(r, kit, seed, f, (u0, u1), t),
        "cave" => cave(r, kit, seed, f, (u0, u1), t),
        _ => rampart(r, kit, seed, f, (u0, u1), t, s),
    }
}

/// Ashlar: two staggered courses of dressed blocks.
fn ashlar(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32, s: f32) {
    // Mortar bed under the blocks keeps the wall solid along its centre.
    block(
        r,
        f,
        ((u0 + u1) / 2.0, 0.0),
        ((u1 - u0) / 2.0, t / 2.0),
        0.5,
        stone(seed ^ 0x40, tone(kit.body, 0.62)),
        0.0,
        1.0,
    );
    for (row, vm) in [(0u64, -t / 4.0), (1, t / 4.0)] {
        for (i, (a, b)) in cuts(seed ^ row, u0, u1, 0.14 * s, 0.3 * s, 0.5 * row as f32)
            .into_iter()
            .enumerate()
        {
            let tint = 0.86 + 0.24 * unit(hash2(seed ^ row, i as i64, 1));
            let base = tone(
                drift(kit.body, unit(hash2(seed, i as i64, row as i64))),
                tint,
            );
            block(
                r,
                f,
                ((a + b) / 2.0, vm),
                ((b - a) / 2.0 - 0.6, t / 4.0 - 0.6),
                1.5,
                stone(seed ^ i as u64, base),
                0.0,
                5.0,
            );
        }
    }
}

/// Timber: three squared beams laid side by side.
fn beams(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    let lane = t / 3.0;
    for i in 0..3 {
        let vm = -t / 2.0 + lane * (i as f32 + 0.5);
        let base = tone(kit.body, 0.88 + 0.2 * unit(hash2(seed, i, 7)));
        block(
            r,
            f,
            ((u0 + u1) / 2.0, vm),
            ((u1 - u0) / 2.0, lane / 2.0 - 0.4),
            1.0,
            wood(seed ^ i as u64, base, f.along(), 0.0),
            0.0,
            3.0,
        );
    }
}

/// Wattle: woven withies between upright stakes.
fn wattle(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32, s: f32) {
    let core = rbox((u0 + u1) / 2.0, 0.0, (u1 - u0) / 2.0, t * 0.4, 1.0);
    r.part(
        f.bounds(u0, u1, t),
        move |x, y| {
            let (u, v) = f.uv(x, y);
            core(u, v)
        },
        wood(seed, tone(kit.body, 0.55), f.along(), 0.0),
        dome(0.0, 2.0, t * 0.4),
        INK,
    );
    let step = t * 0.55;
    let (lo, hi) = (u0, u1);
    let mut u = u0 - step;
    let mut i = 0i64;
    while u < u1 + step {
        let base = tone(kit.body, 0.85 + 0.3 * unit(hash2(seed, i, 2)));
        let (cx, cy) = f.xy(u, 0.0);
        let (ax, ay) = f.along();
        let k = if i % 2 == 0 { 0.5 } else { -0.5 };
        let (dx, dy) = (ax - ay * k, ay + ax * k);
        let n = (dx * dx + dy * dy).sqrt();
        let e = crate::placeholders::paint::obox(
            cx,
            cy,
            t * 0.52,
            t * 0.13,
            t * 0.12,
            (dx / n, dy / n),
        );
        r.part(
            f.bounds(u0, u1, t),
            move |x, y| {
                let (uu, _) = f.uv(x, y);
                e(x, y).max(lo - uu).max(uu - hi)
            },
            wood(seed ^ i as u64, base, (dx / n, dy / n), 0.0),
            dome(2.0, 3.0, t * 0.13),
            SOFT_INK,
        );
        u += step;
        i += 1;
    }
    let mut u = u0 + (0.11 * s).min((u1 - u0) / 2.0);
    while u < u1 {
        let (x, y) = f.xy(u, 0.0);
        r.part(
            Bounds::around(x, y, t * 0.35),
            circle(x, y, t * 0.3),
            wood(seed, kit.joint, f.along(), 0.0),
            dome(6.0, 3.0, t * 0.3),
            INK,
        );
        u += 0.22 * s;
    }
}

/// Palisade: a row of sharpened log ends.
fn palisade(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    let step = t * 0.88;
    let n = ((u1 - u0) / step).round().max(1.0);
    let step = (u1 - u0) / n;
    for i in 0..n as i64 {
        let u = u0 + step * (i as f32 + 0.5);
        let (x, y) = f.xy(u, 0.0);
        let rad = step * 0.54;
        let bark = tone(kit.body, 0.85 + 0.25 * unit(hash2(seed, i, 4)));
        r.part(
            Bounds::around(x, y, rad),
            circle(x, y, rad),
            wood(seed ^ i as u64, bark, (0.0, 1.0), 0.0),
            dome(0.0, rad * 0.5, rad),
            INK,
        );
        let end = tone(kit.joint, 0.9 + 0.2 * unit(hash2(seed, i, 5)));
        let rings = move |px: f32, py: f32, _d: f32| {
            let d = ((px - x) * (px - x) + (py - y) * (py - y)).sqrt();
            tone(end, if (d / 2.4).fract() < 0.3 { 0.86 } else { 1.0 })
        };
        // The sharpened point: a cone rising to a lit tip.
        let (tip, cr) = (rad * 0.9, rad * 0.68);
        r.part(
            Bounds::around(x, y, rad),
            circle(x, y, cr),
            rings,
            move |_, _, d| rad * 0.5 + tip * (-d / cr).clamp(0.0, 1.0),
            SOFT_INK,
        );
    }
}

/// Hedge: a leafy band of overlapping lumps.
fn hedge(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    let (lo, hi) = (u0, u1);
    let core = rbox((u0 + u1) / 2.0, 0.0, (u1 - u0) / 2.0, t * 0.42, t * 0.3);
    let sd = seed ^ 0x4E;
    r.part(
        f.bounds(u0, u1, t),
        move |x, y| {
            let (u, v) = f.uv(x, y);
            core(u, v)
        },
        leafy(seed, tone(kit.body, 0.6), 4.0),
        bumpy(dome(0.0, t * 0.3, t * 0.4), 3.0, move |x, y| {
            fbm(sd, x / 3.0, y / 3.0, 2, None)
        }),
        INK,
    );
    let mut rng = Rng::new(seed);
    let mut u = u0 + rng.range(0.0, t * 0.3);
    while u < u1 {
        let v = rng.range(-0.18, 0.18) * t;
        let (x, y) = f.xy(u, v);
        let rad = t * rng.range(0.3, 0.42);
        let col = tone(drift(kit.body, rng.f()), rng.range(0.85, 1.12));
        let sd = rng.next_u64();
        let lump = blob(sd, x, y, rad, 0.2);
        r.part(
            Bounds::around(x, y, rad * 1.3),
            move |px, py| {
                let (uu, _) = f.uv(px, py);
                lump(px, py).max(lo - uu).max(uu - hi)
            },
            leafy(sd, col, 3.0),
            bumpy(dome(t * 0.2, rad * 0.5, rad), 3.0, move |px, py| {
                fbm(sd, px / 2.6, py / 2.6, 2, None)
            }),
            SOFT_INK,
        );
        u += t * rng.range(0.3, 0.45);
    }
}

/// Drystone: rough stones stacked without mortar, two rows.
fn drystone(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    let mut rng = Rng::new(seed);
    for row in [-0.24f32, 0.24] {
        let mut u = u0 + rng.range(0.0, t * 0.25);
        while u < u1 {
            let rad = t * rng.range(0.26, 0.36);
            let (x, y) = f.xy(u.min(u1 - rad * 0.6), row * t + rng.range(-0.05, 0.05) * t);
            let base = tone(drift(kit.body, rng.f()), rng.range(0.82, 1.12));
            rock(r, rng.next_u64(), (x, y), rad, base, 0.0, rng.f() < 0.35);
            u += rad * 1.45;
        }
    }
    for _ in 0..3 {
        let u = rng.range(u0, u1);
        let (x, y) = f.xy(u, 0.0);
        rock(
            r,
            rng.next_u64(),
            (x, y),
            t * 0.22,
            tone(kit.body, 1.08),
            3.0,
            false,
        );
    }
}

/// Cave rock: an unbroken dark rock band (so every arm reads solid to the
/// importer's arm detector) heaped with rough boulders.
fn cave(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    block(
        r,
        f,
        ((u0 + u1) / 2.0, 0.0),
        ((u1 - u0) / 2.0, t / 2.0),
        t * 0.3,
        stone(seed ^ 0x41, tone(kit.body, 0.7)),
        0.0,
        4.0,
    );
    drystone(r, kit, seed, f, (u0, u1), t);
}

/// Rampart: a flagged wall-walk between crenellated parapets.
fn rampart(r: &mut Relief, kit: &Kit, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32, s: f32) {
    let walk = t * 0.56;
    block(
        r,
        f,
        ((u0 + u1) / 2.0, 0.0),
        ((u1 - u0) / 2.0, t / 2.0),
        0.5,
        stone(seed ^ 0x40, tone(kit.body, 0.62)),
        0.0,
        1.0,
    );
    for (i, (a, b)) in cuts(seed, u0, u1, 0.12 * s, 0.24 * s, 0.3)
        .into_iter()
        .enumerate()
    {
        let tint = 0.9 + 0.18 * unit(mix64(seed ^ i as u64));
        block(
            r,
            f,
            ((a + b) / 2.0, 0.0),
            ((b - a) / 2.0 - 0.5, walk / 2.0),
            1.0,
            stone(seed ^ i as u64, tone(kit.joint, tint)),
            0.0,
            2.0,
        );
    }
    let para = (t - walk) / 2.0;
    for side in [-1.0f32, 1.0] {
        let vm = side * (walk / 2.0 + para / 2.0);
        block(
            r,
            f,
            ((u0 + u1) / 2.0, vm),
            ((u1 - u0) / 2.0, para / 2.0),
            0.5,
            stone(seed ^ 3, kit.body),
            6.0,
            4.0,
        );
        let step = 0.2 * s;
        let n = ((u1 - u0) / step).round().max(1.0);
        let step = (u1 - u0) / n;
        for i in 0..n as i64 {
            let um = u0 + step * (i as f32 + 0.5);
            block(
                r,
                f,
                (um, vm),
                (step * 0.3, para / 2.0 - 0.3),
                1.0,
                stone(seed ^ 5 ^ i as u64, tone(kit.body, 1.06)),
                12.0,
                4.0,
            );
        }
    }
}
