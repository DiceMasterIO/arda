//! Placeholder wall kits: stone, timber, wattle, palisade, hedge, drystone,
//! city wall and cave rock.
//!
//! Every piece is one square. Edge pieces run west to east through the image
//! centre, vertex to vertex. Joint pieces sit on the centre with short arms
//! in their canonical directions (see [`crate::catalog::WallRole`]), so the
//! compositor's choice of corner, tee, cross or end is visible in the render.
//! Bodies are painted by style in [`bands`] on a relief, so every stone,
//! beam, stake and leaf lump is shaded from the top-left and outlined.
// Art generation casts bounded pixel coordinates and cell indices.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

pub mod bands;

use super::material::{metal, stone, wood};
use super::paint::{circle, Rgb};
use super::relief::{dome, Bounds, Relief, INK};
use crate::catalog::WallRole;
use crate::raster::Rgba;
use bands::{block, body, Frame};

/// A wall kit's look.
#[derive(Debug, Clone, Copy)]
pub struct Kit {
    /// Kit name used in the catalogue.
    pub name: &'static str,
    /// Wall thickness as a fraction of a square.
    pub thickness: f32,
    /// Main material colour.
    pub body: Rgb,
    /// Joint (quoin or post) colour.
    pub joint: Rgb,
    /// Block length range as fractions of a square; zero means one beam.
    pub block: (f32, f32),
    /// Wall height in feet (drop-shadow hint).
    pub height_ft: u16,
}

const fn kit(
    name: &'static str,
    thickness: f32,
    body: Rgb,
    joint: Rgb,
    block: (f32, f32),
    height_ft: u16,
) -> Kit {
    Kit {
        name,
        thickness,
        body,
        joint,
        block,
        height_ft,
    }
}

/// Every placeholder kit.
pub const ALL_KITS: [Kit; 8] = [
    kit(
        "stone",
        0.26,
        [150, 146, 136],
        [176, 170, 156],
        (0.14, 0.3),
        10,
    ),
    kit("timber", 0.17, [128, 90, 56], [86, 60, 38], (0.0, 0.0), 9),
    kit("wattle", 0.14, [156, 124, 78], [100, 72, 46], (0.0, 0.0), 6),
    kit(
        "palisade",
        0.24,
        [112, 84, 56],
        [196, 164, 116],
        (0.0, 0.0),
        12,
    ),
    kit("hedge", 0.36, [72, 106, 46], [72, 106, 46], (0.0, 0.0), 6),
    kit(
        "drystone",
        0.24,
        [142, 136, 122],
        [150, 144, 130],
        (0.0, 0.0),
        4,
    ),
    kit(
        "city_wall",
        0.5,
        [150, 144, 132],
        [172, 166, 152],
        (0.12, 0.24),
        25,
    ),
    // Natural cave rock (arda-dungeon): a thick band of rough dark stone.
    kit("cave", 0.32, [92, 86, 78], [104, 98, 88], (0.0, 0.0), 12),
];

/// The two original kits (kept for callers of the first catalogue).
pub const KITS: [Kit; 2] = [ALL_KITS[0], ALL_KITS[1]];

/// Every role each placeholder kit provides.
pub const ROLES: [WallRole; 9] = [
    WallRole::Run,
    WallRole::Door,
    WallRole::Window,
    WallRole::Gate,
    WallRole::Corner,
    WallRole::Tee,
    WallRole::Cross,
    WallRole::End,
    WallRole::Post,
];

const DOOR_WOOD: Rgb = [112, 74, 42];
const IRON: Rgb = [58, 58, 62];

fn rustic(kit: &Kit) -> bool {
    matches!(
        kit.name,
        "wattle" | "hedge" | "drystone" | "palisade" | "cave"
    )
}

/// Paints one kit piece.
#[must_use]
pub fn piece(kit: &Kit, role: WallRole, seed: u64, ppsq: u32) -> Rgba {
    let s = ppsq as f32;
    let c = s / 2.0;
    let t = kit.thickness * s;
    let f = Frame { c, vertical: false };
    let mut r = Relief::new(ppsq, ppsq);
    match role {
        WallRole::Run => body(&mut r, kit, seed, f, 0.0, s, 1.0),
        WallRole::Window if rustic(kit) => {
            body(&mut r, kit, seed, f, 0.0, s * 0.3, 1.0);
            body(&mut r, kit, seed ^ 1, f, s * 0.7, s, 1.0);
            body(&mut r, kit, seed ^ 2, f, s * 0.3, s * 0.7, 0.6);
        }
        WallRole::Window => {
            body(&mut r, kit, seed, f, 0.0, s, 1.0);
            let (hw, hh, col) = if kit.name == "city_wall" {
                (s * 0.03, t * 0.5, [34, 30, 28])
            } else {
                (s * 0.2, t * 0.16, [168, 196, 206])
            };
            block(
                &mut r,
                f,
                (c, 0.0),
                (hw, hh),
                1.0,
                move |_, y, _| {
                    super::paint::mix(
                        col,
                        [220, 232, 236],
                        ((y - c + hh) / (2.0 * hh)).clamp(0.0, 1.0) * 0.4,
                    )
                },
                14.0,
                0.5,
            );
        }
        WallRole::Door => {
            body(&mut r, kit, seed, f, 0.0, s * 0.2, 1.0);
            body(&mut r, kit, seed ^ 1, f, s * 0.8, s, 1.0);
            if rustic(kit) {
                bars(&mut r, seed, f, (s * 0.2, s * 0.8), t.min(s * 0.2));
            } else {
                leaf(&mut r, seed, f, (s * 0.2, s * 0.8), t * 0.45, 0.0);
            }
        }
        WallRole::Gate => {
            if rustic(kit) {
                bars(&mut r, seed, f, (s * 0.06, s * 0.94), t.min(s * 0.2));
            } else {
                leaf(&mut r, seed, f, (s * 0.06, s * 0.5), t * 0.5, 1.0);
                leaf(&mut r, seed ^ 7, f, (s * 0.5, s * 0.94), t * 0.5, 1.0);
            }
            for x in [s * 0.06, s * 0.94] {
                cap(&mut r, kit, seed, (x, c), t * 0.55, true);
            }
        }
        joint_role => {
            let arms = crate::compose::walls::canonical_arms(joint_role);
            let reach = t * 0.7 + s * 0.04;
            let frames = [
                (true, c - reach, c),
                (false, c, c + reach),
                (true, c, c + reach),
                (false, c - reach, c),
            ];
            let seamless = joint_role == WallRole::Post
                && matches!(kit.name, "palisade" | "hedge" | "drystone" | "city_wall");
            for (on, (vertical, u0, u1)) in arms.iter().zip(frames) {
                if *on && !seamless {
                    body(
                        &mut r,
                        kit,
                        seed ^ u64::from(vertical),
                        Frame { c, vertical },
                        u0,
                        u1,
                        1.0,
                    );
                }
            }
            let end = arms.iter().filter(|a| **a).count() == 1;
            if joint_role == WallRole::Post
                && matches!(kit.name, "palisade" | "hedge" | "drystone" | "city_wall")
            {
                // Straight runs of these kits join without a visible post;
                // palisade runs leave a gap at each vertex for one log.
                if kit.name != "city_wall" {
                    cap(&mut r, kit, seed, (c, c), t * 0.4, false);
                }
            } else {
                cap(&mut r, kit, seed, (c, c), t * 0.56, end);
            }
        }
    }
    r.finish(1.0)
}

/// The block at a joint or gate post, in the kit's style.
fn cap(r: &mut Relief, kit: &Kit, seed: u64, (x, y): (f32, f32), p: f32, end: bool) {
    let f = Frame {
        c: y,
        vertical: false,
    };
    match kit.name {
        "stone" | "city_wall" => {
            let p = if kit.name == "city_wall" { p * 1.12 } else { p };
            let round = if end { p * 0.5 } else { 2.0 };
            block(
                r,
                f,
                (x, 0.0),
                (p, p),
                round,
                stone(seed ^ 0x10, kit.joint),
                4.0,
                6.0,
            );
        }
        "timber" | "wattle" => {
            let p = if kit.name == "wattle" { p * 1.3 } else { p };
            block(
                r,
                f,
                (x, 0.0),
                (p, p),
                if end { p * 0.6 } else { 1.5 },
                wood(seed ^ 0x11, kit.joint, (0.0, 1.0), 0.0),
                6.0,
                4.0,
            );
        }
        "palisade" => {
            let rad = if end { p * 1.1 } else { p * 1.2 };
            r.part(
                Bounds::around(x, y, rad * 1.1),
                circle(x, y, rad),
                wood(seed, kit.body, (0.0, 1.0), 0.0),
                dome(0.0, rad * 0.6, rad),
                INK,
            );
            r.part(
                Bounds::around(x, y, rad),
                circle(x, y, rad * 0.7),
                wood(seed ^ 1, kit.joint, (1.0, 0.0), 0.0),
                cone(rad * 0.5, rad * 0.9, rad * 0.7),
                INK,
            );
        }
        "hedge" => {
            let leaf = super::material::leafy(seed, super::paint::tone(kit.body, 1.05), 3.0);
            r.part(
                Bounds::around(x, y, p * 1.3),
                super::paint::blob(seed, x, y, p * 1.05, 0.2),
                leaf,
                dome(p * 0.3, p * 0.6, p),
                INK,
            );
        }
        _ => super::vegetation::rocks::rock(r, seed ^ 0x12, (x, y), p * 1.05, kit.joint, 2.0, true),
    }
}

/// A cone rising `rise` above `z0` to a point at the centre of a shape of
/// radius `rad`.
fn cone(z0: f32, rise: f32, rad: f32) -> impl Fn(f32, f32, f32) -> f32 {
    move |_, _, d| z0 + rise * (-d / rad).clamp(0.0, 1.0)
}

/// A plank door or gate leaf from `u0` to `u1`; `straps` adds iron bands.
fn leaf(r: &mut Relief, seed: u64, f: Frame, (u0, u1): (f32, f32), half: f32, straps: f32) {
    let um = (u0 + u1) / 2.0;
    block(
        r,
        f,
        (um, 0.0),
        ((u1 - u0) / 2.0, half),
        1.0,
        wood(seed ^ 0xD00, DOOR_WOOD, (0.0, 1.0), 8.0),
        2.0,
        2.5,
    );
    if straps > 0.0 {
        for u in [u0 + 3.0, u1 - 3.0] {
            block(
                r,
                f,
                (u, 0.0),
                (1.6, half),
                0.5,
                metal(seed, IRON),
                5.0,
                0.5,
            );
        }
    }
    block(
        r,
        f,
        (um + (u1 - u0) * 0.3, half * 0.5),
        (1.8, 1.8),
        0.5,
        metal(seed ^ 1, [150, 120, 60]),
        5.0,
        0.5,
    );
}

/// A field gate: rails between the posts.
fn bars(r: &mut Relief, seed: u64, f: Frame, (u0, u1): (f32, f32), t: f32) {
    let um = (u0 + u1) / 2.0;
    for v in [-0.3, 0.0, 0.3] {
        block(
            r,
            f,
            (um, v * t),
            ((u1 - u0) / 2.0, 1.6),
            1.0,
            wood(seed ^ 0xBA, [150, 116, 76], (1.0, 0.0), 0.0),
            3.0,
            1.5,
        );
    }
    let a = (u0 + 2.0, -0.35 * t);
    let b = (u1 - 2.0, 0.35 * t);
    let (ax, ay) = f.xy(a.0, a.1);
    let (bx, by) = f.xy(b.0, b.1);
    r.part(
        Bounds::span(ax, ay, bx, by, 3.0),
        super::paint::capsule(ax, ay, bx, by, 1.6),
        wood(seed ^ 0xBB, [136, 104, 68], (1.0, 0.0), 0.0),
        dome(5.0, 1.5, 1.6),
        INK,
    );
}
