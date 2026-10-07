//! Placeholder props, painted top-down on a [`Relief`] so every object has
//! form shading lit from the top-left and an inked outline, like the
//! reference battle maps. No painter darkens anything outside its shapes:
//! cast shadows belong to the lighting pass.
//!
//! Each painter receives the relief, a seed and the square size `s` in
//! pixels. Shapes are laid out in fractions of `s`, so the art scales with
//! the library resolution.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

pub mod civic;
pub mod dungeon;
pub mod farm;
pub mod fire;
pub mod furniture;
pub mod river;
pub mod storage;
pub mod work;
pub mod yard;

use super::material::{metal, stone, wood};
use super::paint::{capsule, circle, rbox, Rgb};
use super::relief::{bevel, dome, Bounds, Ink, Relief, INK, SOFT_INK};

/// Mid oak.
pub const WOOD: Rgb = [138, 98, 60];
/// Pale pine.
pub const PALE_WOOD: Rgb = [178, 138, 90];
/// Dark walnut.
pub const DARK_WOOD: Rgb = [90, 60, 38];
/// Wrought iron.
pub const IRON: Rgb = [78, 78, 82];
/// Brass fittings.
pub const BRASS: Rgb = [196, 158, 70];
/// Burlap sacking.
pub const BURLAP: Rgb = [190, 168, 118];
/// Canvas.
pub const CANVAS: Rgb = [218, 204, 168];
/// Grey stone.
pub const STONE: Rgb = [150, 146, 136];
/// Straw and hay.
pub const HAY: Rgb = [206, 176, 96];
/// Deep red cloth.
pub const RED: Rgb = [150, 44, 38];

/// Along x.
pub const EAST: (f32, f32) = (1.0, 0.0);
/// Along y.
pub const SOUTH: (f32, f32) = (0.0, 1.0);

/// A bevelled rectangle of wood with boards `board` px wide across `dir`.
#[allow(clippy::too_many_arguments)]
pub fn plank_rect(
    r: &mut Relief,
    seed: u64,
    (cx, cy): (f32, f32),
    (hw, hh): (f32, f32),
    base: Rgb,
    dir: (f32, f32),
    board: f32,
    z0: f32,
) {
    let bev = hw.min(hh).clamp(2.0, 5.0);
    r.part(
        Bounds::around(cx, cy, hw.max(hh)),
        rbox(cx, cy, hw, hh, 2.0),
        wood(seed, base, dir, board),
        bevel(z0, 4.0, bev),
        INK,
    );
}

/// A bevelled rectangle with any material.
#[allow(clippy::too_many_arguments)]
pub fn rect(
    r: &mut Relief,
    (cx, cy): (f32, f32),
    (hw, hh): (f32, f32),
    round: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
    rise: f32,
    ink: Ink,
) {
    let bev = hw.min(hh).clamp(1.5, 6.0);
    r.part(
        Bounds::around(cx, cy, hw.max(hh)),
        rbox(cx, cy, hw, hh, round),
        mat,
        bevel(z0, rise, bev),
        ink,
    );
}

/// A domed disc.
pub fn disc(
    r: &mut Relief,
    (cx, cy): (f32, f32),
    rad: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
    rise: f32,
    ink: Ink,
) {
    r.part(
        Bounds::around(cx, cy, rad),
        circle(cx, cy, rad),
        mat,
        dome(z0, rise, rad),
        ink,
    );
}

/// A flat-topped disc with a rounded rim.
pub fn puck(
    r: &mut Relief,
    (cx, cy): (f32, f32),
    rad: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
    ink: Ink,
) {
    r.part(
        Bounds::around(cx, cy, rad),
        circle(cx, cy, rad),
        mat,
        bevel(z0, 4.0, (rad * 0.3).clamp(1.5, 5.0)),
        ink,
    );
}

/// A ring (annulus) between radii `inner` and `outer`.
pub fn ring(
    r: &mut Relief,
    (cx, cy): (f32, f32),
    inner: f32,
    outer: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
) {
    let (a, b) = (circle(cx, cy, outer), circle(cx, cy, inner));
    let mid = (inner + outer) / 2.0;
    let half = (outer - inner) / 2.0;
    r.part(
        Bounds::around(cx, cy, outer),
        move |x, y| a(x, y).max(-b(x, y)),
        mat,
        move |x, y, _| {
            let d = ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() - mid;
            let t = (1.0 - (d / half).abs()).clamp(0.0, 1.0);
            z0 + 3.0 * t
        },
        SOFT_INK,
    );
}

/// A rounded rod from `a` to `b` (a round profile across its width).
pub fn rod(
    r: &mut Relief,
    a: (f32, f32),
    b: (f32, f32),
    rad: f32,
    mat: impl Fn(f32, f32, f32) -> Rgb,
    z0: f32,
    ink: Ink,
) {
    r.part(
        Bounds::span(a.0, a.1, b.0, b.1, rad),
        capsule(a.0, a.1, b.0, b.1, rad),
        mat,
        dome(z0, rad, rad),
        ink,
    );
}

/// A wooden rod.
pub fn pole(r: &mut Relief, seed: u64, a: (f32, f32), b: (f32, f32), rad: f32, base: Rgb, z0: f32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
    rod(
        r,
        a,
        b,
        rad,
        wood(seed, base, (dx / l, dy / l), 0.0),
        z0,
        INK,
    );
}

/// An iron fitting.
pub fn iron_rect(r: &mut Relief, seed: u64, c: (f32, f32), h: (f32, f32), z0: f32) {
    rect(r, c, h, 1.0, metal(seed, IRON), z0, 1.5, SOFT_INK);
}

/// A stone block.
pub fn stone_rect(r: &mut Relief, seed: u64, c: (f32, f32), h: (f32, f32), base: Rgb, z0: f32) {
    rect(r, c, h, 3.0, stone(seed, base), z0, 6.0, INK);
}

/// A small flame or glowing coal: bright and unshaded (flat height).
pub fn glow(r: &mut Relief, seed: u64, c: (f32, f32), rad: f32, z0: f32) {
    r.part(
        Bounds::around(c.0, c.1, rad),
        super::paint::blob(seed, c.0, c.1, rad, 0.2),
        move |x, y, _| {
            let d = ((x - c.0) * (x - c.0) + (y - c.1) * (y - c.1)).sqrt() / rad;
            let n = crate::noise::fbm(seed, x / 4.0, y / 4.0, 2, None);
            super::paint::mix([255, 236, 150], [214, 70, 24], (d * 0.9 + n * 0.3).min(1.0))
        },
        super::relief::level(z0),
        super::relief::NO_INK,
    );
}
