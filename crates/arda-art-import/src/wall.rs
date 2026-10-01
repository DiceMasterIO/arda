//! Wall-piece orientation against the canonical arms (README, "Walls"):
//! edge pieces run west–east through the centre; joints have arms
//! `corner` E+S, `tee` E+S+W, `cross` all, `end` E, `post` E+W.

// Window bounds are fractions of the piece size; casts are exact.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use arda_tactical::catalog::WallRole;
use arda_tactical::Rgba;

/// Arm bits.
const E: u8 = 1;
const S: u8 = 2;
const W: u8 = 4;
const N: u8 = 8;

/// The canonical arm mask of a role.
#[must_use]
pub fn canonical(role: WallRole) -> u8 {
    match role {
        WallRole::Run | WallRole::Door | WallRole::Window | WallRole::Gate | WallRole::Post => {
            E | W
        }
        WallRole::Corner => E | S,
        WallRole::Tee => E | S | W,
        WallRole::Cross => E | S | W | N,
        WallRole::End => E,
    }
}

/// The mask after one clockwise quarter turn (E→S→W→N→E).
#[must_use]
pub fn turn(mask: u8) -> u8 {
    ((mask << 1) | (mask >> 3)) & 0xF
}

/// Names of the arms in a mask.
#[must_use]
pub fn arm_names(mask: u8) -> String {
    let names: Vec<&str> = [(E, "E"), (S, "S"), (W, "W"), (N, "N")]
        .iter()
        .filter(|(b, _)| mask & b != 0)
        .map(|(_, n)| *n)
        .collect();
    if names.is_empty() {
        "none".into()
    } else {
        names.join("+")
    }
}

/// Mean alpha (0–1) over a window given in fractions of the size.
fn coverage(img: &Rgba, (x0, x1): (f32, f32), (y0, y1): (f32, f32)) -> f32 {
    let (w, h) = (img.width as f32, img.height as f32);
    let (ax, bx) = ((x0 * w) as u32, ((x1 * w) as u32).min(img.width));
    let (ay, by) = ((y0 * h) as u32, ((y1 * h) as u32).min(img.height));
    let mut sum = 0u64;
    let mut n = 0u64;
    for y in ay..by {
        for x in ax..bx {
            sum += u64::from(img.get(x, y)[3]);
            n += 1;
        }
    }
    if n == 0 {
        0.0
    } else {
        sum as f32 / (n as f32 * 255.0)
    }
}

/// How far solid wall reaches from the centre along each axis (E, S, W,
/// N), in pixels: the run of band slices at least half covered.
#[must_use]
pub fn reaches(img: &Rgba) -> [u32; 4] {
    let s = img.width.min(img.height);
    let c = s / 2;
    let half = (s * 8 / 100).max(2);
    let slice = |along: u32, dir: usize| {
        let mut sum = 0u32;
        let mut n = 0u32;
        for t in c.saturating_sub(half)..(c + half).min(s) {
            let (x, y) = match dir {
                0 => (c + along, t),
                1 => (t, c + along),
                2 => (c - along, t),
                _ => (t, c - along),
            };
            sum += u32::from(img.get(x, y)[3]);
            n += 1;
        }
        sum * 2 >= n * 255
    };
    std::array::from_fn(|dir| {
        let max = if dir < 2 { s - 1 - c } else { c };
        (0..=max).take_while(|&k| slice(k, dir)).count() as u32
    })
}

/// The arms a joint shows: the directions whose reach stands out from the
/// shortest, or `None` when all reaches are alike (a plain post).
#[must_use]
pub fn arms(img: &Rgba) -> Option<u8> {
    let r = reaches(img);
    let (lo, hi) = (r.iter().min().copied()?, r.iter().max().copied()?);
    let s = img.width.min(img.height);
    if hi - lo < (s / 20).max(2) {
        return None;
    }
    let cut = lo + (hi - lo) / 2;
    let bits = [E, S, W, N];
    Some((0..4).filter(|&k| r[k] > cut).fold(0, |m, k| m | bits[k]))
}

/// Whether an edge piece reaches both the west and east borders: some row
/// near the axis is solid in the outer 4 % at each end.
#[must_use]
pub fn reaches_ends(img: &Rgba) -> bool {
    let (w, h) = (img.width, img.height);
    let strip = (w / 25).max(1);
    let solid_at = |x0: u32| {
        (h * 3 / 10..h * 7 / 10).any(|y| {
            let sum: u32 = (x0..x0 + strip).map(|x| u32::from(img.get(x, y)[3])).sum();
            sum * 2 >= strip * 255
        })
    };
    solid_at(0) && solid_at(w - strip)
}

/// The outcome of an orientation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orientation {
    /// Already canonical (or symmetric, so any turn is canonical).
    Canonical,
    /// Turn clockwise by this many quarter turns to make it canonical.
    Turn(u8),
    /// No turn matches; the message says what was seen.
    Mismatch(String),
}

/// Checks a fitted piece against its role's canonical arms. Edge pieces
/// must run west–east (more wall along the horizontal axis than the
/// vertical one); joints must show their role's arms in some rotation.
#[must_use]
pub fn check(img: &Rgba, role: WallRole) -> Orientation {
    if role.is_edge() {
        let h = coverage(img, (0.0, 1.0), (0.4, 0.6));
        let v = coverage(img, (0.4, 0.6), (0.0, 1.0));
        return if v > h * 1.3 {
            Orientation::Turn(1)
        } else {
            Orientation::Canonical
        };
    }
    if role == WallRole::Cross {
        // Every turn of a cross is a cross.
        return Orientation::Canonical;
    }
    let want = canonical(role);
    let Some(seen) = arms(img) else {
        return Orientation::Canonical;
    };
    let mut m = seen;
    for k in 0..4u8 {
        if m == want {
            return if k == 0 {
                Orientation::Canonical
            } else {
                Orientation::Turn(k)
            };
        }
        m = turn(m);
    }
    Orientation::Mismatch(format!(
        "arms {} do not match a {:?} in any rotation (canonical {})",
        arm_names(seen),
        role,
        arm_names(want)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn turning_cycles() {
        assert_eq!(turn(E), S);
        assert_eq!(turn(N), E);
        assert_eq!(turn(turn(turn(turn(E | S)))), E | S);
    }

    #[test]
    fn every_placeholder_piece_is_canonical() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        let lib = arda_tactical::Library::load(&dir).unwrap();
        let mut n = 0;
        for a in lib.catalog.assets.iter().filter(|a| a.wall.is_some()) {
            let role = a.wall.as_ref().unwrap().role;
            let img = lib.image(&a.id).unwrap();
            assert_eq!(check(img, role), Orientation::Canonical, "{}", a.id);
            // A piece drawn the wrong way round is detected and turned back.
            if role.is_edge() {
                assert!(reaches_ends(img), "{}", a.id);
            }
            if role != WallRole::Cross && (role.is_edge() || arms(img).is_some()) {
                let turned = img.rotated(1);
                let Orientation::Turn(k) = check(&turned, role) else {
                    panic!("{} turned was not detected", a.id);
                };
                assert_eq!(check(&turned.rotated(k), role), Orientation::Canonical);
            }
            n += 1;
        }
        assert!(n >= 35);
    }
}
