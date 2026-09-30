//! Tree canopies seen from above, painted in layers back to front: a dark
//! base mass, then rings of leaf clusters rising and brightening towards
//! the crown. Each cluster is a domed lump with a fine bumpy height field,
//! so the relief shading picks out leaves lit from the top-left. Conifers
//! use radiating needle sprays; willows cascade; dead trees are bare
//! branches.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::noise::fbm;
use crate::placeholders::material::{drift, heading, leafy, straw, wood, Rng};
use crate::placeholders::paint::{blob, circle, taper, tone, Rgb};
use crate::placeholders::relief::{bumpy, dome, Bounds, Ink, Relief, INK, SOFT_INK};

/// Cluster outline: softer than the silhouette ink.
const LEAF_INK: Ink = Ink {
    width: 1.6,
    strength: 0.5,
};

/// One leaf cluster: a lumpy dome whose colour is also painted lighter on
/// its top-left and darker on its bottom-right, with leaf-scale bumps.
fn cluster(r: &mut Relief, seed: u64, c: (f32, f32), rad: f32, leaf: Rgb, z0: f32, ink: Ink) {
    let bump = move |x: f32, y: f32| fbm(seed ^ 0xB0, x / 2.4, y / 2.4, 2, None);
    let mat = leafy(seed, leaf, (rad * 0.3).max(2.5));
    r.part(
        Bounds::around(c.0, c.1, rad * 1.25),
        blob(seed, c.0, c.1, rad, 0.22),
        move |x, y, d| {
            let lit = (-(x - c.0) - (y - c.1)) / (rad * 1.4);
            tone(mat(x, y, d), 1.0 + 0.28 * lit.clamp(-1.0, 1.0))
        },
        bumpy(dome(z0, rad * 0.8, rad), 1.4, bump),
        ink,
    );
}

/// A broadleaf canopy. `leaf` sets the hue, `airy` thins the crown and
/// `fruit` sprinkles coloured fruit over the top.
pub fn broadleaf(r: &mut Relief, seed: u64, leaf: Rgb, airy: bool, fruit: Option<Rgb>) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let big = w.min(h) * 0.46;
    let mut rng = Rng::new(seed);
    // Trunk and limbs, only glimpsed through gaps.
    for i in 0..5 {
        let (dx, dy) = heading(i as f32 / 5.0 + rng.range(-0.05, 0.05));
        let tip = (c.0 + dx * big * 0.6, c.1 + dy * big * 0.6);
        r.part(
            Bounds::span(c.0, c.1, tip.0, tip.1, 6.0),
            taper(c, tip, big * 0.07, big * 0.02),
            wood(seed, [86, 64, 44], (dx, dy), 0.0),
            dome(0.0, 3.0, 3.0),
            INK,
        );
    }
    let base_rad = if airy { big * 0.74 } else { big * 0.82 };
    cluster(r, seed ^ 1, c, base_rad, tone(leaf, 0.42), 2.0, INK);
    let rings: [(f32, u32, f32, f32, f32); 4] = if airy {
        [
            (0.7, 14, 0.2, 0.66, 10.0),
            (0.48, 11, 0.19, 0.84, 18.0),
            (0.26, 7, 0.18, 1.0, 26.0),
            (0.05, 2, 0.17, 1.14, 34.0),
        ]
    } else {
        [
            (0.68, 16, 0.24, 0.66, 10.0),
            (0.46, 12, 0.23, 0.84, 19.0),
            (0.25, 7, 0.22, 1.02, 28.0),
            (0.05, 2, 0.2, 1.16, 37.0),
        ]
    };
    for (ring, n, size, k, z) in rings {
        let spin = rng.f();
        for i in 0..n {
            let (dx, dy) = heading(spin + (i as f32 + rng.range(-0.3, 0.3)) / n as f32);
            let off = big * ring * rng.range(0.85, 1.15);
            let p = (c.0 + dx * off, c.1 + dy * off);
            // Clusters on the lit (north-west) side are a touch brighter.
            let side = 1.0 - 0.06 * (dx + dy);
            let col = tone(drift(leaf, rng.f()), k * side * rng.range(0.92, 1.08));
            cluster(
                r,
                rng.next_u64(),
                p,
                big * size * rng.range(0.85, 1.15),
                col,
                z,
                LEAF_INK,
            );
        }
    }
    // Sunlit sprigs on the crown.
    for _ in 0..14 {
        let (dx, dy) = rng.dir();
        let off = big * rng.range(0.0, 0.55);
        let p = (c.0 + dx * off - big * 0.08, c.1 + dy * off - big * 0.08);
        cluster(
            r,
            rng.next_u64(),
            p,
            big * rng.range(0.07, 0.12),
            tone(leaf, 1.22),
            40.0,
            SOFT_INK,
        );
    }
    if let Some(f) = fruit {
        for _ in 0..16 {
            let (dx, dy) = rng.dir();
            let off = big * rng.range(0.1, 0.8);
            let p = (c.0 + dx * off, c.1 + dy * off);
            r.part(
                Bounds::around(p.0, p.1, 4.0),
                circle(p.0, p.1, 2.6),
                move |_, _, _| f,
                dome(44.0, 2.0, 2.6),
                SOFT_INK,
            );
        }
    }
}

/// A conifer: tiers of needle sprays radiating from the crown. `tiers`
/// controls density; `spruce` gives a darker, denser, bluer tree.
pub fn conifer(r: &mut Relief, seed: u64, leaf: Rgb, spruce: bool) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let big = w.min(h) * 0.46;
    let mut rng = Rng::new(seed);
    r.part(
        Bounds::around(c.0, c.1, big),
        blob(seed, c.0, c.1, big * 0.6, 0.15),
        leafy(seed, tone(leaf, 0.5), 5.0),
        dome(0.0, 6.0, big * 0.6),
        INK,
    );
    let tiers: [(f32, u32, f32, f32); 4] = if spruce {
        [
            (1.0, 30, 0.66, 0.0),
            (0.8, 24, 0.8, 8.0),
            (0.58, 18, 0.95, 16.0),
            (0.34, 11, 1.1, 24.0),
        ]
    } else {
        [
            (1.0, 24, 0.7, 0.0),
            (0.78, 19, 0.86, 9.0),
            (0.55, 14, 1.0, 18.0),
            (0.3, 8, 1.14, 27.0),
        ]
    };
    for (reach, n, k, z) in tiers {
        let spin = rng.f();
        for i in 0..n {
            let (dx, dy) = heading(spin + (i as f32 + rng.range(-0.3, 0.3)) / n as f32);
            let l = big * reach * rng.range(0.82, 1.0);
            let tip = (c.0 + dx * l, c.1 + dy * l);
            let root = (c.0 + dx * l * 0.35, c.1 + dy * l * 0.35);
            let col = tone(
                drift(leaf, rng.f()),
                k * rng.range(0.9, 1.08) * (1.0 - 0.05 * (dx + dy)),
            );
            let wid = big * if spruce { 0.15 } else { 0.13 } * reach.sqrt();
            let sd = rng.next_u64();
            let spray = taper(root, tip, wid, 2.0);
            // Feathered edges: needles break up the spray's outline.
            let feather = move |x: f32, y: f32| {
                spray(x, y) + 3.0 * (fbm(sd, x / 2.0, y / 2.0, 2, None) - 0.5) * 2.0
            };
            let bump = move |x: f32, y: f32| fbm(sd ^ 1, x / 1.8, y / 1.8, 1, None);
            r.part(
                Bounds::span(root.0, root.1, tip.0, tip.1, wid + 4.0),
                feather,
                leafy(sd, col, 2.0),
                bumpy(dome(z, wid * 0.6, wid), 1.2, bump),
                LEAF_INK,
            );
        }
    }
    r.part(
        Bounds::around(c.0, c.1, 8.0),
        circle(c.0, c.1, big * 0.06),
        leafy(seed, tone(leaf, 1.2), 2.0),
        dome(42.0, 4.0, big * 0.06),
        SOFT_INK,
    );
}

/// A weeping willow: a mounded crown of pale, streaky clusters with a
/// fringe of drooping strands spilling over its edge.
pub fn willow(r: &mut Relief, seed: u64, leaf: Rgb) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let big = w.min(h) * 0.46;
    let mut rng = Rng::new(seed);
    // The fringe first, so the crown overlaps its roots.
    for _ in 0..120 {
        let (dx, dy) = rng.dir();
        let a = (c.0 + dx * big * 0.62, c.1 + dy * big * 0.62);
        let l = big * rng.range(0.22, 0.38);
        let bend = rng.range(-0.25, 0.25);
        let b = (a.0 + (dx + bend * dy) * l, a.1 + (dy - bend * dx) * l);
        let col = tone(drift(leaf, rng.f()), rng.range(0.62, 0.9));
        let wid = big * rng.range(0.022, 0.034);
        r.part(
            Bounds::span(a.0, a.1, b.0, b.1, wid + 2.0),
            taper(a, b, wid, 1.1),
            straw(rng.next_u64(), col, (dx, dy)),
            dome(2.0, 1.5, wid),
            SOFT_INK,
        );
    }
    cluster(r, seed, c, big * 0.7, tone(leaf, 0.5), 4.0, INK);
    for (ring, n, size, k, z) in [
        (0.5, 14, 0.24, 0.78, 12.0),
        (0.28, 8, 0.22, 0.94, 20.0),
        (0.06, 3, 0.2, 1.1, 28.0),
    ] {
        let spin = rng.f();
        for i in 0..n {
            let (dx, dy) = heading(spin + (i as f32 + rng.range(-0.3, 0.3)) / n as f32);
            let off = big * ring * rng.range(0.85, 1.15);
            let p = (c.0 + dx * off, c.1 + dy * off);
            let rad = big * size * rng.range(0.85, 1.15);
            let col = tone(
                drift(leaf, rng.f()),
                k * (1.0 - 0.06 * (dx + dy)) * rng.range(0.92, 1.08),
            );
            let sd = rng.next_u64();
            let streaks = straw(sd, col, (dx, dy));
            r.part(
                Bounds::around(p.0, p.1, rad * 1.25),
                blob(sd, p.0, p.1, rad, 0.22),
                move |x, y, d| {
                    let lit = (-(x - p.0) - (y - p.1)) / (rad * 1.4);
                    tone(streaks(x, y, d), 1.0 + 0.25 * lit.clamp(-1.0, 1.0))
                },
                dome(z, rad * 0.7, rad),
                LEAF_INK,
            );
        }
    }
}

/// A dead tree: bare, forking grey-brown branches.
pub fn dead(r: &mut Relief, seed: u64) {
    let (w, h) = (r.width as f32, r.height as f32);
    let c = (w / 2.0, h / 2.0);
    let big = w.min(h) * 0.46;
    let mut rng = Rng::new(seed);
    let bark = [104, 92, 80];
    let mut limbs = Vec::new();
    for i in 0..6 {
        let (dx, dy) = heading(i as f32 / 6.0 + rng.range(-0.06, 0.06));
        limbs.push((c, (dx, dy), big * rng.range(0.55, 0.75), big * 0.075, 0u32));
    }
    while let Some((a, (dx, dy), l, wid, depth)) = limbs.pop() {
        let b = (a.0 + dx * l, a.1 + dy * l);
        let col = tone(bark, rng.range(0.85, 1.1));
        r.part(
            Bounds::span(a.0, a.1, b.0, b.1, wid),
            taper(a, b, wid, wid * 0.55),
            wood(rng.next_u64(), col, (dx, dy), 0.0),
            dome(20.0 - 5.0 * depth as f32, wid, wid),
            INK,
        );
        if depth < 2 {
            for turn in [-0.07, 0.07] {
                let (tx, ty) = heading(turn + rng.range(-0.03, 0.03));
                let nd = (dx * tx - dy * ty, dx * ty + dy * tx);
                limbs.push((b, nd, l * 0.55, wid * 0.55, depth + 1));
            }
        }
    }
    r.part(
        Bounds::around(c.0, c.1, big * 0.12),
        circle(c.0, c.1, big * 0.1),
        wood(seed, tone(bark, 0.9), (1.0, 0.0), 0.0),
        dome(22.0, 4.0, big * 0.1),
        INK,
    );
}
