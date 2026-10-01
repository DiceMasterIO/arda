//! Yards and crofts (goal 64: no visible repetition). A home's yard is
//! dressed from a pool chosen by its function and wealth, with a count and
//! positions from its own stream, along its back and sides and a square
//! further out. A croft parcel draws its own density and a secondary use
//! (a woodpile in a paddock, hay drying in a meadow), so neighbouring
//! parcels differ. Both depend only on the plan, never on the window.

use super::exterior::{all, beside_at, kind, prop};
use super::interior::GProp;
use crate::function::BuildingFunction as F;
use crate::plan::grid::{Kind, Side};
use crate::plan::{Building, TownPlan, WealthLevel as W};
use crate::rng::{mix, Rng};

type Pool = &'static [(&'static str, u32)];

fn yard_pool(f: F, w: W) -> Pool {
    match (f, w) {
        (F::Farmhouse, _) => &[
            ("prop.hay_bale", 4),
            ("prop.woodpile", 3),
            ("prop.wheelbarrow", 2),
            ("prop.bucket", 2),
            ("prop.trough", 2),
            ("prop.cart", 1),
            ("prop.haycart", 1),
            ("prop.well", 1),
        ],
        (F::Cottage, _) | (F::House, W::Poor) => &[
            ("prop.woodpile", 4),
            ("prop.bucket", 3),
            ("prop.sacks", 2),
            ("prop.hay_bale", 1),
            ("prop.wheelbarrow", 1),
            ("prop.barrel", 1),
        ],
        (F::House, W::Wealthy) | (F::Manor, _) => &[
            ("prop.bench", 3),
            ("veg.bush_flowering", 3),
            ("prop.barrel", 1),
            ("prop.woodpile", 2),
            ("prop.statue", 1),
            ("prop.well", 1),
        ],
        (F::House, _) => &[
            ("prop.woodpile", 4),
            ("prop.barrel", 3),
            ("prop.crate", 2),
            ("prop.bucket", 2),
            ("prop.wheelbarrow", 1),
            ("prop.bench", 1),
        ],
        _ => &[
            ("prop.woodpile", 3),
            ("prop.barrel", 3),
            ("prop.crate", 3),
            ("prop.sacks", 2),
        ],
    }
}

fn draw(rng: &mut Rng, pool: Pool) -> &'static str {
    let total: u32 = pool.iter().map(|p| p.1).sum();
    let mut t = u32::try_from(rng.next_u64() % u64::from(total.max(1))).unwrap_or(0);
    for &(id, w) in pool {
        if t < w {
            return id;
        }
        t -= w;
    }
    pool.first().map_or("prop.woodpile", |p| p.0)
}

/// Whether a function's yard is dressed here.
#[must_use]
pub fn dressed(f: F) -> bool {
    matches!(
        f,
        F::House
            | F::Cottage
            | F::Farmhouse
            | F::Manor
            | F::Bakery
            | F::Brewery
            | F::Workshop
            | F::Tannery
    )
}

/// A home's or workshop's yard: 0–5 props along its back and sides, one or
/// two squares out, and sometimes a bench or a barrel by the front.
pub fn home(plan: &TownPlan, b: &Building, rng: &mut Rng, out: &mut Vec<GProp>) {
    let pool = yard_pool(b.function, b.wealth_level);
    let most = match b.function {
        F::Farmhouse => 6,
        F::Cottage => 4,
        _ => 5,
    };
    let n = rng.index(most + 1);
    let back = b.front.opposite();
    let sides = match b.front {
        Side::North | Side::South => [Side::East, Side::West],
        _ => [Side::North, Side::South],
    };
    let mut used: Vec<(i64, i64)> = Vec::new();
    for _ in 0..n {
        let id = draw(rng, pool);
        let side = if rng.chance(0.65) {
            back
        } else {
            sides[rng.index(2)]
        };
        let depth = if rng.chance(0.3) { 2 } else { 1 };
        let span = match side {
            Side::North | Side::South => b.rect.w(),
            _ => b.rect.h(),
        };
        let start =
            i64::try_from(rng.index(usize::try_from(span.max(1)).unwrap_or(1))).unwrap_or(0);
        let spot = (0..span).find_map(|k| {
            let at = beside_at(
                plan,
                b,
                side,
                (start + k) % span.max(1),
                depth,
                &[Kind::Yard],
            )?;
            (!used.contains(&at) && all(plan, at.0, at.1, 1, 1, &[Kind::Yard])).then_some(at)
        });
        if let Some((x, y)) = spot {
            used.push((x, y));
            let rot = if rng.chance(0.5) { 0 } else { 90 };
            let (w, h) = super::interior::size(id);
            let fits = if rot == 90 { (h, w) } else { (w, h) };
            if all(plan, x, y, fits.0, fits.1, &[Kind::Yard]) {
                out.push(prop(id, x, y, rot));
            }
        }
    }
    if b.wealth_level != W::Poor && rng.chance(0.25) {
        let id = if rng.chance(0.6) {
            "prop.bench"
        } else {
            "prop.barrel"
        };
        let span = match b.front {
            Side::North | Side::South => b.rect.w(),
            _ => b.rect.h(),
        };
        let k = i64::try_from(rng.index(usize::try_from(span.max(1)).unwrap_or(1))).unwrap_or(0);
        if let Some((x, y)) = beside_at(plan, b, b.front, k, 1, &[Kind::Front]) {
            out.push(prop(id, x, y, 0));
        }
    }
}

/// A parcel's density in permille of the base rates, 500–1500.
fn density(p: u64) -> u64 {
    500 + mix(p ^ 0xDE45) % 1001
}

/// Croft dressing: fruit trees in rows in orchards, troughs and tall grass
/// in paddocks, flowers and drying hay in meadows, tools in kitchen gardens,
/// stacks and carts in yards, bushes and young trees on the rim. A prop
/// stands only on squares of its own parcel.
pub fn croft(plan: &TownPlan, x: i64, y: i64, pick: u64, out: &mut Vec<GProp>) {
    use crate::plan::croft::{parcel, rim, use_of, Use};
    let s = plan.seed;
    let p = parcel(s, x, y);
    let own = |w: i64, h: i64| {
        all(plan, x, y, w, h, &[Kind::Croft])
            && (y..y + h).all(|b| (x..x + w).all(|a| parcel(s, a, b) == p))
    };
    if rim(s, |a, b| kind(plan, a, b) == Kind::Open, x, y) {
        if pick < 70 {
            out.push(prop("veg.bush", x, y, 0));
        } else if pick < 85 && own(2, 2) {
            out.push(prop("veg.tree_birch", x, y, 0));
        } else if pick < 130 {
            out.push(prop("veg.tall_grass", x, y, 0));
        }
        return;
    }
    let rot = if p & 2 == 0 { 0 } else { 90 };
    // Rates scale with the parcel's density; `t(r)` is the scaled rate.
    let dens = density(p);
    let t = |r: u64| r * dens / 1000;
    let extra = mix(p ^ 0x5EC0) % 4;
    match use_of(p) {
        Use::Orchard => {
            let (ox, oy) = (
                i64::try_from(p % 4).unwrap_or(0),
                i64::try_from((p >> 8) % 4).unwrap_or(0),
            );
            let step = 3 + i64::try_from(extra % 2).unwrap_or(0);
            if (x - ox).rem_euclid(step) == 0
                && (y - oy).rem_euclid(step) == 0
                && pick < 850
                && own(2, 2)
            {
                out.push(prop("veg.tree_fruit", x, y, 0));
            } else if pick >= 990 && own(1, 1) {
                let id = [
                    "prop.wheelbarrow",
                    "prop.bucket",
                    "prop.sacks",
                    "prop.crate",
                ][usize::try_from(extra).unwrap_or(0)];
                out.push(prop(id, x, y, 0));
            } else if (900..900 + t(40)).contains(&pick) {
                out.push(prop("veg.tall_grass", x, y, 0));
            }
        }
        Use::Paddock => {
            if pick < t(4) && own(2, 1) {
                out.push(prop("prop.trough", x, y, 0));
            } else if (100..100 + t(26)).contains(&pick) {
                out.push(prop("veg.tall_grass", x, y, 0));
            } else if extra == 0 && (200..200 + t(4)).contains(&pick) && own(1, 1) {
                out.push(prop("prop.hay_bale", x, y, 0));
            } else if (300..300 + t(6)).contains(&pick) {
                out.push(prop("veg.bush", x, y, 0));
            }
        }
        Use::Meadow => {
            if pick < t(35) {
                out.push(prop("veg.flower_patch", x, y, 0));
            } else if (100..100 + t(25)).contains(&pick) {
                out.push(prop("veg.tall_grass", x, y, 0));
            } else if extra == 1 && (200..200 + t(6)).contains(&pick) && own(1, 1) {
                out.push(prop("prop.hay_bale", x, y, 0));
            } else if (300..300 + t(4)).contains(&pick) {
                out.push(prop("veg.bush_flowering", x, y, 0));
            }
        }
        Use::Kitchen => {
            if pick < t(5) && own(1, 1) {
                out.push(prop("prop.bucket", x, y, 0));
            } else if (100..100 + t(3)).contains(&pick) && own(1, 1) {
                out.push(prop("prop.wheelbarrow", x, y, rot));
            } else if (200..200 + t(3)).contains(&pick) && own(1, 1) {
                out.push(prop("prop.sacks", x, y, 0));
            } else if extra < 2 && (300..300 + t(6)).contains(&pick) {
                out.push(prop("veg.flower_patch", x, y, 0));
            }
        }
        Use::Yard => {
            let ids = [
                "prop.hay_bale",
                "prop.woodpile",
                "prop.cart",
                "prop.crate",
                "prop.barrel",
                "prop.wheelbarrow",
                "prop.bucket",
                "prop.haycart",
            ];
            if pick < t(24) {
                // The parcel favours some of the kinds.
                let k = usize::try_from((mix(pick ^ p) % 3 + extra * 2) % 8).unwrap_or(0);
                let id = ids[k];
                let (w, h) = super::interior::size(id);
                let (w, h) = if rot == 90 { (h, w) } else { (w, h) };
                if own(w.max(2), h.max(2)) {
                    out.push(prop(id, x, y, rot));
                }
            }
        }
    }
}
