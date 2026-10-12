//! Placeholder effect art (1.0 task 05): transparent textures that tile and
//! decals that do not, for a consumer to draw over the tactical tiles.
//!
//! Every pixel is a pure function of the seed and the id. Textures use
//! periodic noise so opposite edges meet; decals fade to nothing before the
//! border. Colour is straight (not premultiplied) alpha.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::{base, PPSQ};
use crate::catalog::{Asset, AssetClass, Layer, EFFECT_DECAL_TAG, EFFECT_TEXTURE_TAG};
use crate::noise::{fbm, hash_str};
use crate::raster::Rgba;

/// Variants made of every effect.
pub const VARIANTS: u32 = 3;
/// Side of every effect, in squares.
pub const SQUARES: u32 = 2;

/// Tileable effect textures (`effect.<name>.<n>`).
pub const TEXTURES: [&str; 8] = [
    "frost", "fire", "acid", "ash", "web", "grease", "fog", "water",
];
/// Single-patch effect decals (`effect.<name>.<n>`).
pub const DECALS: [&str; 1] = ["scorch_mark"];

type Rgb = [f32; 3];

fn smooth(lo: f32, hi: f32, v: f32) -> f32 {
    let t = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Periodic noise at `cells` lattice cells across the tile.
fn pn(seed: u64, u: f32, v: f32, cells: i64, octaves: u32) -> f32 {
    fbm(
        seed,
        u * cells as f32,
        v * cells as f32,
        octaves,
        Some(cells),
    )
}

/// `(colour, alpha in 0..=1)` of the named texture at tile position `(u, v)`.
fn texture_px(name: &str, seed: u64, u: f32, v: f32) -> (Rgb, f32) {
    let big = pn(seed, u, v, 4, 3);
    let fine = pn(seed ^ 0x11, u, v, 16, 2);
    match name {
        "frost" => {
            let crystal = 1.0 - (pn(seed ^ 0x21, u, v, 12, 2) - 0.5).abs() * 9.0;
            let a = smooth(0.38, 0.62, big) * 0.62 + crystal.clamp(0.0, 1.0) * 0.3;
            (
                lerp([186.0, 222.0, 246.0], [246.0, 252.0, 255.0], fine),
                a.min(0.92),
            )
        }
        "fire" => {
            let heat = (big * 0.7 + fine * 0.3 + 0.1 * pn(seed ^ 0x31, u, v, 8, 1)).min(1.0);
            let c = lerp(
                [196.0, 40.0, 8.0],
                [255.0, 226.0, 96.0],
                smooth(0.35, 0.8, heat),
            );
            (c, smooth(0.28, 0.6, heat) * 0.92)
        }
        "acid" => {
            let bubble = smooth(0.62, 0.7, pn(seed ^ 0x41, u, v, 10, 1));
            let a = 0.45 + 0.35 * big + 0.15 * bubble;
            (
                lerp(
                    [90.0, 176.0, 30.0],
                    [186.0, 238.0, 70.0],
                    bubble.max(fine * 0.5),
                ),
                a,
            )
        }
        "ash" => {
            let speck = smooth(0.55, 0.8, pn(seed ^ 0x51, u, v, 32, 1));
            let a = 0.35 + 0.4 * big + 0.2 * speck;
            (
                lerp([70.0, 68.0, 66.0], [150.0, 146.0, 140.0], fine),
                a.min(0.9),
            )
        }
        "web" => {
            let l1 = (pn(seed ^ 0x61, u, v, 6, 2) - 0.5).abs();
            let l2 = (pn(seed ^ 0x62, u, v, 6, 2) - 0.5).abs();
            let strand = (1.0 - (l1.min(l2) / 0.018)).clamp(0.0, 1.0);
            ([238.0, 238.0, 232.0], strand * 0.9)
        }
        "grease" => {
            let sheen = smooth(0.6, 0.8, fine);
            (
                lerp([54.0, 44.0, 22.0], [120.0, 100.0, 54.0], sheen),
                0.42 + 0.36 * smooth(0.3, 0.7, big),
            )
        }
        "fog" => {
            let soft = pn(seed, u, v, 3, 3);
            (
                [226.0, 232.0, 238.0],
                0.08 + 0.55 * smooth(0.25, 0.75, soft),
            )
        }
        _ => {
            // water
            let ripple = 1.0 - (pn(seed ^ 0x71, u, v, 8, 2) - 0.5).abs() * 7.0;
            (
                lerp(
                    [44.0, 104.0, 176.0],
                    [170.0, 214.0, 244.0],
                    ripple.clamp(0.0, 1.0) * 0.7,
                ),
                0.4 + 0.3 * smooth(0.3, 0.7, big),
            )
        }
    }
}

/// `(colour, alpha)` of the scorch-mark decal at `(u, v)` in `[0, 1)`.
fn decal_px(seed: u64, u: f32, v: f32) -> (Rgb, f32) {
    let (dx, dy) = (u - 0.5, v - 0.5);
    let r = (dx * dx + dy * dy).sqrt() * 2.0;
    let wobble = pn(seed, u, v, 5, 3) - 0.5;
    let edge = r + wobble * 0.5;
    let body = 1.0 - smooth(0.55, 0.9, edge);
    let core = 1.0 - smooth(0.0, 0.5, edge);
    let soot = pn(seed ^ 0x81, u, v, 14, 2);
    let c = lerp([40.0, 34.0, 30.0], [16.0, 14.0, 13.0], core.max(soot * 0.4));
    // The decal always ends before the border.
    let guard = 1.0 - smooth(0.9, 1.0, r);
    (c, body * (0.55 + 0.35 * core) * guard)
}

fn paint(side: u32, px: impl Fn(f32, f32) -> (Rgb, f32)) -> Rgba {
    let mut img = Rgba::new(side, side);
    for y in 0..side {
        for x in 0..side {
            let (c, a) = px(
                (x as f32 + 0.5) / side as f32,
                (y as f32 + 0.5) / side as f32,
            );
            let b = |v: f32| (v + 0.5).clamp(0.0, 255.0) as u8;
            img.set(x, y, [b(c[0]), b(c[1]), b(c[2]), b(a * 255.0)]);
        }
    }
    img
}

/// Every placeholder effect: catalogue record and image.
#[must_use]
pub fn assets(seed: u64) -> Vec<(Asset, Rgba)> {
    let mut out = Vec::new();
    let kinds = TEXTURES
        .iter()
        .map(|n| (*n, true))
        .chain(DECALS.iter().map(|n| (*n, false)));
    for (name, texture) in kinds {
        for v in 0..VARIANTS {
            let id = format!("effect.{name}.{v}");
            let mut a = base(
                &id,
                AssetClass::Effect,
                "effects",
                SQUARES,
                SQUARES,
                Layer::Floor,
                seed,
            );
            a.tileable = texture;
            a.tags.free = vec![if texture {
                EFFECT_TEXTURE_TAG
            } else {
                EFFECT_DECAL_TAG
            }
            .into()];
            // A texture is tiled and a decal placed by its consumer, in any
            // of four turns; neither blocks, covers or casts a shadow.
            let s = hash_str(seed, &id);
            let img = if texture {
                paint(SQUARES * PPSQ, |u, w| texture_px(name, s, u, w))
            } else {
                paint(SQUARES * PPSQ, |u, w| decal_px(s, u, w))
            };
            out.push((a, img));
        }
    }
    out
}
