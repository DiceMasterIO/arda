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

/// Most props a working yard holds.
pub const YARD_MOST: usize = 4;
/// Yard squares per prop: a yard of fewer squares holds fewer props.
const SQUARES_PER_PROP: usize = 10;

/// What a building's trade leaves in its yard, each piece once, in order
/// of importance. Pieces flagged `true` stand against the building's wall
/// (stacks, barrels, a bench); the others one square out in the open (a
/// cart, a trough, a well).
fn yard_kit(f: F, w: W) -> &'static [(&'static str, bool)] {
    match (f, w) {
        (F::Farmhouse, _) => &[
            ("prop.hay_bale", true),
            ("prop.trough", false),
            ("prop.woodpile", true),
            ("prop.cart", false),
        ],
        (F::Barn, _) => &[("prop.hay_bale", true), ("prop.haycart", false)],
        (F::Stable, _) => &[("prop.trough", false), ("prop.hay_bale", true)],
        (F::Smithy, _) => &[
            ("prop.grindstone", false),
            ("prop.trough", false),
            ("prop.woodpile", true),
        ],
        (F::Bakery, _) => &[("prop.woodpile", true), ("prop.sacks", true)],
        (F::Brewery, _) => &[
            ("prop.barrel", true),
            ("prop.cask_rack", true),
            ("prop.cart", false),
        ],
        (F::Tannery, _) => &[("prop.trough", false), ("prop.barrel", true)],
        (F::Workshop, _) => &[
            ("prop.crate", true),
            ("prop.woodpile", true),
            ("prop.sacks", true),
        ],
        (F::Mill, _) => &[("prop.sacks", true), ("prop.cart", false)],
        (F::Inn | F::Tavern, _) => &[
            ("prop.barrel", true),
            ("prop.trough", false),
            ("prop.crate", true),
        ],
        (F::Manor, _) | (F::House, W::Wealthy) => &[
            ("prop.bench", true),
            ("prop.well", false),
            ("prop.woodpile", true),
        ],
        (F::Cottage, _) | (F::House, W::Poor) => &[("prop.woodpile", true), ("prop.bucket", true)],
        (F::House, _) => &[("prop.woodpile", true), ("prop.barrel", true)],
        _ => &[],
    }
}

/// Squares a doorway keeps clear: the square outside each door of the
/// buildings near `b`, its two neighbours along the wall and the two
/// squares straight out from it.
fn door_paths(plan: &TownPlan, b: &Building) -> Vec<(i64, i64)> {
    let near = b.rect.grown(6);
    let mut out = Vec::new();
    for o in plan.buildings.iter().filter(|o| o.rect.overlaps(&near)) {
        for d in &o.doors {
            let (ox, oy) = d.outside();
            let (sx, sy) = d.side.step();
            out.extend([
                (ox, oy),
                (ox + sx, oy + sy),
                (ox + 2 * sx, oy + 2 * sy),
                (ox + sy, oy + sx),
                (ox - sy, oy - sx),
            ]);
        }
    }
    out
}

/// A working yard dressed for its building's trade (the WFC leaves yards
/// bare): at most [`YARD_MOST`] props and one per [`SQUARES_PER_PROP`] yard
/// squares, each piece of the trade's kit once, never on a doorway's path
/// and never touching another of this yard's props. A pure function of the
/// plan and the building, so every window agrees.
pub fn working(plan: &TownPlan, b: &Building, out: &mut Vec<GProp>) {
    let kit = yard_kit(b.function, b.wealth_level);
    if kit.is_empty() {
        return;
    }
    let plot = b.plot.map(|p| p.0 + 1);
    let own = |x: i64, y: i64| {
        kind(plan, x, y) == Kind::Yard
            && plot.is_none_or(|p| plan.grid.gidx(x, y).is_some_and(|k| plan.grid.plot[k] == p))
    };
    let r = b.rect;
    // Yard squares within two of the building, with their ring: 1 touches
    // its wall, 2 stands one square out.
    let mut spots: Vec<(u64, i64, i64, i64)> = Vec::new();
    for y in r.y0 - 2..r.y1 + 2 {
        for x in r.x0 - 2..r.x1 + 2 {
            let ring = (r.x0 - x).max(x - r.x1 + 1).max(r.y0 - y).max(y - r.y1 + 1);
            if (1..=2).contains(&ring) && own(x, y) {
                let h = crate::rng::hash_i(plan.seed ^ 0x7A2D ^ b.id.0, x, y);
                spots.push((h, ring, x, y));
            }
        }
    }
    let yard = spots.len();
    spots.sort_unstable();
    let most = kit
        .len()
        .min(YARD_MOST)
        .min(yard.div_ceil(SQUARES_PER_PROP));
    let clear = door_paths(plan, b);
    // The kit's order rotates by building, so neighbours lead with
    // different pieces.
    let turn = usize::try_from(mix(plan.seed ^ b.id.0 ^ 0x7A2E) % 64).unwrap_or(0);
    let mut taken: Vec<[i64; 4]> = Vec::new();
    let mut placed = 0;
    for i in 0..kit.len() {
        if placed >= most {
            break;
        }
        // The first piece is the trade's mark and stays first.
        let (id, wall) = if i == 0 {
            kit[0]
        } else {
            kit[1 + (i - 1 + turn) % (kit.len() - 1).max(1)]
        };
        let (w, h) = super::interior::size(id);
        let fit = spots.iter().find_map(|&(_, ring, x, y)| {
            if ring != if wall { 1 } else { 2 } {
                return None;
            }
            [(0, w, h), (90, h, w)]
                .into_iter()
                .find_map(|(rot, fw, fh)| {
                    let e = [x, y, x + fw, y + fh];
                    let free = (y..y + fh).all(|yy| (x..x + fw).all(|xx| own(xx, yy)))
                        && all(plan, x, y, fw, fh, &[Kind::Yard])
                        && !(y..y + fh).any(|yy| (x..x + fw).any(|xx| clear.contains(&(xx, yy))))
                        && taken
                            .iter()
                            .all(|t| e[0] > t[2] || t[0] > e[2] || e[1] > t[3] || t[1] > e[3]);
                    free.then_some((rot, e))
                })
        });
        if let Some((rot, e)) = fit {
            taken.push(e);
            out.push(prop(id, e[0], e[1], rot));
            placed += 1;
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
