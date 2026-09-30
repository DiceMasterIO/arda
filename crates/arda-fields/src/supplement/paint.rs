//! Painters for the supplementary placeholder art.
// Art generation casts bounded pixel coordinates.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use arda_tactical::compose::walls::canonical_arms;
use arda_tactical::noise::{hash2, mix as mix64, unit};
use arda_tactical::placeholders::paint::{
    blob, capsule, circle, ellipse, fill, mix, painterly, rbox, tone, Rgb,
};
use arda_tactical::raster::Rgba;
use arda_tactical::WallRole;

/// Recolours a texture: each pixel keeps its relative brightness but takes
/// `tint`; `keep` of the original colour survives. Tiling is preserved.
#[must_use]
pub fn tinted(src: &Rgba, tint: Rgb, keep: f32) -> Rgba {
    let mut n = 0.0_f64;
    let mut sum = 0.0_f64;
    for px in src.data.as_chunks::<4>().0 {
        sum += f64::from(luma(px));
        n += 1.0;
    }
    let mean = (sum / n.max(1.0)) as f32;
    let mut out = src.clone();
    for px in out.data.as_chunks_mut::<4>().0 {
        let k = luma(px) / mean.max(1.0);
        for c in 0..3 {
            let t = f32::from(tint[c]) * k;
            let v = f32::from(px[c]) * keep + t * (1.0 - keep);
            px[c] = v.round().clamp(0.0, 255.0) as u8;
        }
        px[3] = 255;
    }
    out
}

fn luma(px: &[u8; 4]) -> f32 {
    0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2])
}

/// Arms of a wall piece `[n, e, s, w]`; edge pieces run east–west.
fn arms(role: WallRole) -> [bool; 4] {
    if role.is_edge() {
        [false, true, false, true]
    } else {
        canonical_arms(role)
    }
}

/// A hedge piece: overlapping leafy lumps along each arm, about a square
/// thick, with a ragged outline. A gate piece is a timber five-bar gate
/// hung between two posts, leaving the gap in the hedge readable.
pub fn hedge(img: &mut Rgba, seed: u64, role: WallRole) {
    let s = img.width as f32;
    let c = s / 2.0;
    if role == WallRole::Gate {
        let wood: Rgb = [168, 128, 80];
        for x in [0.06, 0.94] {
            fill(
                img,
                circle(x * s, c, 0.1 * s),
                painterly(seed, [104, 76, 46], 5.0),
            );
        }
        for dy in [-0.09, -0.03, 0.03, 0.09] {
            fill(
                img,
                rbox(c, c + dy * s, 0.42 * s, 0.018 * s, 1.0),
                painterly(seed ^ 5, wood, 5.0),
            );
        }
        fill(
            img,
            capsule(0.12 * s, c + 0.09 * s, 0.88 * s, c - 0.09 * s, 0.02 * s),
            painterly(seed ^ 6, wood, 5.0),
        );
        return;
    }
    let a = arms(role);
    let dirs = [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)];
    let greens: [Rgb; 3] = [[54, 88, 38], [70, 104, 44], [48, 78, 40]];
    let mut lumps = vec![(c, c, 0.4 * s)];
    for (k, on) in a.iter().enumerate() {
        if !on {
            continue;
        }
        let (dx, dy) = dirs[k];
        for (n, t) in [0.17_f32, 0.34, 0.5].into_iter().enumerate() {
            let h = hash2(seed, k as i64, n as i64);
            let off = (unit(h) - 0.5) * 0.14 * s;
            let r = (0.34 + 0.1 * unit(mix64(h ^ 20))) * s;
            lumps.push((c + dx * t * s - dy * off, c + dy * t * s + dx * off, r));
        }
    }
    for (i, (x, y, r)) in lumps.into_iter().enumerate() {
        let sd = seed ^ (i as u64).wrapping_mul(0x9E37);
        let leaf = greens[i % 3];
        let shade = move |px: f32, py: f32, d: f32| {
            let g = unit(hash2(sd, (px / 4.0) as i64, (py / 4.0) as i64));
            let lit = ((x - px) + (y - py)) / (2.0 * r);
            let base = mix(
                tone(leaf, 0.75),
                tone(leaf, 1.3),
                (0.5 * g + 0.5 * lit).clamp(0.0, 1.0),
            );
            tone(base, if d > -3.0 { 0.7 } else { 1.0 })
        };
        fill(img, blob(sd, x, y, r, 0.22), shade);
    }
}

/// A round hay bale or stook.
pub fn hay_bale(img: &mut Rgba, seed: u64) {
    let s = img.width as f32;
    let straw: Rgb = [210, 178, 96];
    fill(
        img,
        circle(s / 2.0, s / 2.0, 0.34 * s),
        painterly(seed, straw, 5.0),
    );
    for r in [0.24, 0.14] {
        fill(
            img,
            |x, y| (((x - s / 2.0).powi(2) + (y - s / 2.0).powi(2)).sqrt() - r * s).abs() - 1.2,
            |_, _, _| tone(straw, 0.7),
        );
    }
}

/// A wooden trough holding water, two squares long.
pub fn trough(img: &mut Rgba, seed: u64) {
    let (w, h) = (img.width as f32, img.height as f32);
    fill(
        img,
        rbox(w / 2.0, h / 2.0, 0.45 * w, 0.26 * h, 3.0),
        painterly(seed, [122, 86, 52], 6.0),
    );
    fill(
        img,
        rbox(w / 2.0, h / 2.0, 0.41 * w, 0.16 * h, 2.0),
        |x, _, _| mix([70, 104, 120], [96, 132, 146], (x / w).fract()),
    );
}

/// A millstone: a grooved grey disc with an eye, two squares across.
pub fn millstone(img: &mut Rgba, seed: u64) {
    let s = img.width as f32;
    let c = s / 2.0;
    fill(
        img,
        circle(c, c, 0.4 * s),
        painterly(seed, [150, 146, 138], 7.0),
    );
    let r = std::f32::consts::FRAC_1_SQRT_2;
    let spokes = [
        (1.0, 0.0),
        (r, r),
        (0.0, 1.0),
        (-r, r),
        (-1.0, 0.0),
        (-r, -r),
        (0.0, -1.0),
        (r, -r),
    ];
    for (dx, dy) in spokes {
        fill(
            img,
            capsule(
                c + dx * 0.1 * s,
                c + dy * 0.1 * s,
                c + dx * 0.37 * s,
                c + dy * 0.37 * s,
                1.5,
            ),
            |_, _, _| [112, 108, 102],
        );
    }
    fill(img, circle(c, c, 0.07 * s), |_, _, _| [60, 56, 52]);
}

/// An undershot waterwheel seen from above: a paddle drum across its axle,
/// two squares along the axle.
pub fn waterwheel(img: &mut Rgba, seed: u64) {
    let (w, h) = (img.width as f32, img.height as f32);
    let wood: Rgb = [110, 78, 48];
    fill(
        img,
        rbox(w / 2.0, h / 2.0, 0.47 * w, 0.06 * h, 2.0),
        |_, _, _| tone(wood, 0.6),
    );
    for x in [0.12, 0.88] {
        fill(
            img,
            rbox(x * w, h / 2.0, 0.04 * w, 0.46 * h, 2.0),
            painterly(seed, wood, 5.0),
        );
    }
    for k in 0..7 {
        let y = (0.1 + 0.8 * k as f32 / 6.0) * h;
        fill(
            img,
            rbox(w / 2.0, y, 0.38 * w, 0.035 * h, 1.0),
            painterly(seed ^ 9, tone(wood, 1.15), 5.0),
        );
    }
}

/// A grazing animal: `sheep` is a fleecy cream lump, otherwise a cow.
pub fn beast(img: &mut Rgba, seed: u64, sheep: bool) {
    let (w, h) = (img.width as f32, img.height as f32);
    if sheep {
        let body: Rgb = [226, 220, 204];
        fill(
            img,
            blob(seed, w * 0.5, h * 0.55, 0.26 * w, 0.18),
            painterly(seed, body, 3.0),
        );
        fill(
            img,
            ellipse(w * 0.5, h * 0.22, 0.09 * w, 0.11 * h),
            |_, _, _| [48, 42, 38],
        );
    } else {
        let hsh = hash2(seed, 5, 5);
        let body: Rgb = if unit(hsh) < 0.5 {
            [120, 76, 44]
        } else {
            [70, 58, 50]
        };
        fill(
            img,
            ellipse(w * 0.5, h * 0.56, 0.3 * w, 0.3 * h),
            painterly(seed, body, 6.0),
        );
        fill(
            img,
            ellipse(w * 0.5, h * 0.17, 0.16 * w, 0.09 * h),
            painterly(seed ^ 1, tone(body, 0.85), 4.0),
        );
        fill(img, circle(w * 0.42, h * 0.5, 0.08 * w), |_, _, _| {
            [224, 218, 206]
        });
    }
}

/// A hen: small, pale, with a red comb.
pub fn hen(img: &mut Rgba, seed: u64) {
    let s = img.width as f32;
    fill(
        img,
        ellipse(s * 0.5, s * 0.55, 0.14 * s, 0.18 * s),
        painterly(seed, [222, 206, 176], 3.0),
    );
    fill(img, circle(s * 0.5, s * 0.36, 0.05 * s), |_, _, _| {
        [190, 50, 40]
    });
}

/// A hay cart: a timber bed heaped with hay, one by two squares.
pub fn haycart(img: &mut Rgba, seed: u64) {
    let (w, h) = (img.width as f32, img.height as f32);
    fill(
        img,
        rbox(w / 2.0, h * 0.55, 0.42 * w, 0.38 * h, 3.0),
        painterly(seed, [118, 84, 50], 6.0),
    );
    fill(
        img,
        blob(seed, w / 2.0, h * 0.55, 0.36 * w, 0.2),
        painterly(seed ^ 2, [206, 176, 96], 5.0),
    );
    fill(
        img,
        capsule(w * 0.5, h * 0.15, w * 0.5, h * 0.02, 2.0),
        |_, _, _| [96, 68, 40],
    );
}

/// A stone hearth ring with embers.
pub fn hearth(img: &mut Rgba, seed: u64) {
    let s = img.width as f32;
    fill(
        img,
        rbox(s / 2.0, s / 2.0, 0.42 * s, 0.42 * s, 6.0),
        painterly(seed, [128, 122, 114], 5.0),
    );
    fill(img, circle(s / 2.0, s / 2.0, 0.24 * s), |x, y, _| {
        mix(
            [60, 30, 20],
            [236, 128, 48],
            unit(hash2(seed, (x / 4.0) as i64, (y / 4.0) as i64)),
        )
    });
}

/// A clump of tall grass stems.
pub fn tall_grass(img: &mut Rgba, seed: u64) {
    let s = img.width as f32;
    let c = s / 2.0;
    for i in 0..22 {
        let h = hash2(seed, i, 3);
        let v = (unit(h) * 2.0 - 1.0, unit(mix64(h ^ 12)) * 2.0 - 1.0);
        let n = (v.0 * v.0 + v.1 * v.1).sqrt().max(0.05);
        let l = (0.18 + 0.22 * unit(mix64(h ^ 20))) * s;
        let col = mix([108, 132, 56], [168, 170, 86], unit(mix64(h ^ 40)));
        let tip = (c + v.0 / n * l, c + v.1 / n * l);
        fill(img, capsule(c, c, tip.0, tip.1, 1.4), move |_, _, _| col);
    }
}
