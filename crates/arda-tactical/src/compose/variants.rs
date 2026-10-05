//! Which asset, rotation and mirror a placement draws.
//!
//! An [`AssetRef::Id`] names a variant *family*: the asset and its extra
//! takes `<id>.alt1`, `<id>.alt2`, … ([`Library::family`]). One member is
//! drawn per placement, picked by a hash of the seed and the placement's
//! position, so a map full of barrels does not repeat one barrel. An id
//! that names a take (`prop.barrel.alt2`) pins exactly that asset.
//!
//! A [`AssetRef::Query`] first picks among the families of its matches by
//! the seed and placement index (as it always has), then among the chosen
//! family's matching takes by position.
//!
//! Floor-layer props (bridge decks, dock planks) of one family whose
//! squares touch form a *structure* that draws a single take, keyed by
//! the structure's smallest world anchor ([`resolve_all`]), so a bridge
//! is not a patchwork of takes. Pinned takes join no structure.
//!
//! Assets carrying the free tag [`ROT_FREE`] are also turned and mirrored
//! by hash when the placement leaves rotation 0 and no mirror.
//!
//! Vegetation on a square footprint anchored at its centre is also given a
//! hashed *pose* when the placement leaves rotation 0 and no mirror: a
//! scale between [`SCALE_MIN_PCT`] and [`SCALE_MAX_PCT`] percent in steps of
//! [`SCALE_STEP_PCT`] and, half
//! the time, a transpose (a mirror across the top-left to bottom-right
//! diagonal, which keeps a top-left highlight in the top-left). The free tag
//! [`FIXED_POSE`] opts an asset out. The pose is purely visual: the
//! footprint, and so blocking and cover, stay as catalogued.
//!
//! The position is the placement's anchor in world squares (`origin` plus
//! the local coordinates) when the layout has an origin, so neighbouring
//! windows that share a placement draw the same take and turn; without an
//! origin the placement index is hashed in too.

use crate::catalog::{Asset, AssetClass, Layer};
use crate::layout::{AssetRef, Placement, TacticalLayout};
use crate::library::{family_base, Library};
use crate::noise::hash2;
use std::collections::BTreeMap;

/// Free tag that lets the compositor turn and mirror an asset by hash:
/// for round or radial things (barrels, crates, trees, bushes, rocks)
/// whose look and meaning do not depend on which way they face.
pub const ROT_FREE: &str = "rot_free";

/// Free tag that keeps a vegetation asset at its catalogued size and
/// orientation (no hashed scale or transpose).
pub const FIXED_POSE: &str = "fixed_pose";

/// Smallest hashed vegetation scale, percent.
pub const SCALE_MIN_PCT: u8 = 85;
/// Largest hashed vegetation scale, percent.
pub const SCALE_MAX_PCT: u8 = 115;
/// Step between hashed vegetation scales, percent: coarse enough that each
/// asset has few distinct sprites to cache (7 scales × 2 transposes).
pub const SCALE_STEP_PCT: u8 = 5;

/// Sub-square steps per square when hashing a placement's position.
const POS_STEPS: f32 = 64.0;

/// What one placement draws.
#[derive(Debug, Clone, Copy)]
pub struct Resolved<'a> {
    /// The asset.
    pub asset: &'a Asset,
    /// Clockwise quarter turns.
    pub turns: u8,
    /// Mirrored before turning.
    pub mirror: bool,
    /// Drawn size in percent of the footprint (100: as catalogued).
    pub scale_pct: u8,
    /// Mirrored across the top-left to bottom-right diagonal, before the
    /// mirror and turns.
    pub transpose: bool,
}

/// Whether `a` may be drawn at `rotation` degrees, mirrored or not.
#[must_use]
pub fn fits(a: &Asset, rotation: u16, mirror: bool) -> bool {
    a.rotations.contains(&rotation) && (!mirror || a.mirror)
}

/// Every asset an [`AssetRef`] could resolve to: an id's whole variant
/// family, or every match of a query.
#[must_use]
pub fn candidates<'a>(lib: &'a Library, r: &AssetRef) -> Vec<&'a Asset> {
    match r {
        AssetRef::Id(id) => lib.family(id),
        AssetRef::Query { class, tags } => lib.query(*class, tags),
    }
}

/// A placement's anchor in world 1/64 squares (`origin` plus local).
#[allow(clippy::cast_possible_truncation)] // rounded map positions
fn world_steps(layout: &TacticalLayout, p: &Placement) -> (i64, i64) {
    let step = |v: f32| (v * POS_STEPS).round() as i64;
    let (ox, oy) = layout.world_origin();
    let steps = POS_STEPS as i64;
    (ox * steps + step(p.x), oy * steps + step(p.y))
}

/// The take key of an anchor at world steps `(x, y)` held by placement
/// `index`: the position, plus the index when the layout has no origin.
fn key_at(layout: &TacticalLayout, (x, y): (i64, i64), index: usize, seed: u64) -> u64 {
    let index = match layout.origin {
        Some(_) => 0,
        None => i64::try_from(index).unwrap_or(0),
    };
    hash2(hash2(seed ^ 0x7A57, x, y), index, 0)
}

/// The hash key of placement `index` of `layout`: its world position, plus
/// its index when the layout has no origin.
fn key(layout: &TacticalLayout, p: &Placement, index: usize, seed: u64) -> u64 {
    key_at(layout, world_steps(layout, p), index, seed)
}

fn nth<'a>(all: &[&'a Asset], h: u64) -> Option<&'a Asset> {
    let n = all.len() as u64;
    if n == 0 {
        return None;
    }
    all.get(usize::try_from(h % n).ok()?).copied()
}

/// The takes a placement may draw, before the pick.
struct Choice<'a> {
    /// Its family's takes it may draw: for an id those allowing its
    /// rotation and mirror, for a query the chosen family's matches.
    members: Vec<&'a Asset>,
    /// The family whose structure it joins: set for floor-layer props of a
    /// family with several takes, unless the id pins one take.
    floor: Option<&'a str>,
}

fn choice<'a>(lib: &'a Library, p: &Placement, index: usize, seed: u64) -> Option<Choice<'a>> {
    let all = candidates(lib, &p.asset);
    let (members, family): (Vec<&Asset>, Option<&str>) = match &p.asset {
        AssetRef::Id(id) => {
            let members = all
                .iter()
                .copied()
                .filter(|a| fits(a, p.rotation, p.mirror))
                .collect();
            // A pinned take (`x.alt2`) is its own family of one.
            let family = (family_base(id) == id.as_str())
                .then(|| all.first().map(|a| family_base(&a.id)))
                .flatten();
            (members, family)
        }
        AssetRef::Query { .. } => {
            let family = pick_family(&all, seed, index)?;
            let members = all
                .into_iter()
                .filter(|a| family_base(&a.id) == family)
                .collect();
            (members, Some(family))
        }
    };
    let floor = family.filter(|_| {
        members.len() > 1
            && members
                .iter()
                .all(|a| a.layer == Layer::Floor && a.class == AssetClass::Prop)
    });
    Some(Choice { members, floor })
}

/// The squares `a` covers when placed by `p` at its own rotation and
/// mirror: those whose centre lies inside the footprint, or the anchor's
/// square when none does.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn squares(a: &Asset, p: &Placement) -> Vec<(i64, i64)> {
    let anchor = a.anchor_or_centre();
    let (mut ax, mut ay) = (anchor.x, anchor.y);
    let (mut w, mut h) = (a.footprint.w as f32, a.footprint.h as f32);
    if p.mirror {
        ax = w - ax;
    }
    for _ in 0..(p.rotation / 90) % 4 {
        // Same clockwise turn as the compositor: (x, y) -> (h - y, x).
        (ax, ay) = (h - ay, ax);
        (w, h) = (h, w);
    }
    let (x0, y0) = (p.x - ax, p.y - ay);
    // Centre i + 0.5 lies in [v0, v1) iff i in [ceil(v0 - 0.5), ceil(v1 - 0.5)).
    let lo = |v: f32| (v - 0.5).ceil() as i64;
    let (xs, ys) = (lo(x0)..lo(x0 + w), lo(y0)..lo(y0 + h));
    let out: Vec<(i64, i64)> = ys.flat_map(|y| xs.clone().map(move |x| (x, y))).collect();
    if out.is_empty() {
        vec![(p.x.floor() as i64, p.y.floor() as i64)]
    } else {
        out
    }
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Groups the floor placements into structures: placements of one family
/// whose covered squares touch (share a square or a side). Returns each
/// placement's structure as the index of a representative member.
fn structures(layout: &TacticalLayout, choices: &[Option<Choice<'_>>]) -> Vec<Option<usize>> {
    let mut parent: Vec<usize> = (0..choices.len()).collect();
    let mut at: BTreeMap<(&str, i64, i64), usize> = BTreeMap::new();
    for (i, c) in choices.iter().enumerate() {
        let (Some(c), Some(p)) = (c, layout.placements.get(i)) else {
            continue;
        };
        let (Some(family), Some(a)) = (c.floor, c.members.first()) else {
            continue;
        };
        for (x, y) in squares(a, p) {
            for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                if let Some(&j) = at.get(&(family, x + dx, y + dy)) {
                    let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
            at.entry((family, x, y)).or_insert(i);
        }
    }
    (0..choices.len())
        .map(|i| {
            choices[i]
                .as_ref()
                .and_then(|c| c.floor)
                .map(|_| root(&mut parent, i))
        })
        .collect()
}

/// The take every member of one structure draws, or `None` when no take
/// of the family suits every member (each then draws the family's base
/// asset if it may, else its own pick).
fn structure_take<'a>(
    layout: &TacticalLayout,
    choices: &[Option<Choice<'a>>],
    group: &[usize],
    seed: u64,
) -> Option<&'a Asset> {
    let first = choices.get(*group.first()?)?.as_ref()?;
    let common: Vec<&Asset> = first
        .members
        .iter()
        .copied()
        .filter(|a| {
            group.iter().all(|&j| {
                let (Some(Some(c)), Some(p)) = (choices.get(j), layout.placements.get(j)) else {
                    return false;
                };
                fits(a, p.rotation, p.mirror) && c.members.iter().any(|m| m.id == a.id)
            })
        })
        .collect();
    // The structure's key: its smallest anchor in world steps (ties to the
    // lowest index), plus that index when the layout has no origin.
    let (pos, index) = group
        .iter()
        .filter_map(|&j| Some((world_steps(layout, layout.placements.get(j)?), j)))
        .min()?;
    nth(&common, key_at(layout, pos, index, seed))
}

/// Resolves every placement of `layout`, in layout order. Floor-layer
/// props of one family whose squares touch form a *structure* (a bridge,
/// a boardwalk) and draw one take between them; see [`resolve`].
#[must_use]
pub fn resolve_all<'a>(
    lib: &'a Library,
    layout: &TacticalLayout,
    seed: u64,
) -> Vec<Option<Resolved<'a>>> {
    let choices: Vec<Option<Choice<'a>>> = layout
        .placements
        .iter()
        .enumerate()
        .map(|(i, p)| choice(lib, p, i, seed))
        .collect();
    let groups = structures(layout, &choices);
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, g) in groups.iter().enumerate() {
        if let Some(r) = g {
            members.entry(*r).or_default().push(i);
        }
    }
    let takes: BTreeMap<usize, Option<&Asset>> = members
        .iter()
        .map(|(r, group)| (*r, structure_take(layout, &choices, group, seed)))
        .collect();
    (0..layout.placements.len())
        .map(|i| {
            let c = choices.get(i)?.as_ref()?;
            let take = groups
                .get(i)
                .copied()
                .flatten()
                .map(|r| takes.get(&r).copied().flatten());
            finish(layout, i, seed, c, take)
        })
        .collect()
}

/// Draws placement `index` from its choice: the structure's take when
/// `take` is `Some(Some(_))`, the base asset (or its own pick) when the
/// structure found no common take (`Some(None)`), else its own pick.
fn finish<'a>(
    layout: &TacticalLayout,
    index: usize,
    seed: u64,
    c: &Choice<'a>,
    take: Option<Option<&'a Asset>>,
) -> Option<Resolved<'a>> {
    let p = layout.placements.get(index)?;
    let h = key(layout, p, index, seed);
    let asset = match take {
        Some(Some(a)) => a,
        Some(None) => c
            .floor
            .and_then(|f| c.members.iter().copied().find(|a| a.id == f))
            .filter(|a| fits(a, p.rotation, p.mirror))
            .or_else(|| nth(&c.members, h))?,
        None => nth(&c.members, h)?,
    };
    let (rotation, mirror) = turn(asset, p, h);
    let (scale_pct, transpose) = pose(asset, p, h);
    Some(Resolved {
        asset,
        turns: u8::try_from(rotation / 90).unwrap_or(0),
        mirror,
        scale_pct,
        transpose,
    })
}

/// Resolves placement `index` of `layout` to an asset, rotation and mirror.
///
/// A floor-layer prop with several takes draws its structure's take,
/// which needs the whole layout: prefer [`resolve_all`] when resolving
/// every placement.
#[must_use]
pub fn resolve<'a>(
    lib: &'a Library,
    layout: &TacticalLayout,
    index: usize,
    seed: u64,
) -> Option<Resolved<'a>> {
    let p = layout.placements.get(index)?;
    let c = choice(lib, p, index, seed)?;
    if c.floor.is_some() {
        return resolve_all(lib, layout, seed)
            .into_iter()
            .nth(index)
            .flatten();
    }
    finish(layout, index, seed, &c, None)
}

/// The hashed scale and transpose of a vegetation placement left at
/// rotation 0 unmirrored, or `(100, false)`. Only square footprints
/// anchored at their centre qualify, so the sprite stays centred on the
/// placement and a transpose keeps the footprint; [`FIXED_POSE`] opts out.
/// `h` is the take pick's key, so overlapping windows agree.
fn pose(a: &Asset, p: &Placement, h: u64) -> (u8, bool) {
    let eligible = a.class == AssetClass::Vegetation
        && a.footprint.w == a.footprint.h
        && a.anchor_or_centre() == a.footprint_centre()
        && p.rotation == 0
        && !p.mirror
        && !a.tags.free.iter().any(|t| t == FIXED_POSE);
    if !eligible {
        return (100, false);
    }
    let h = hash2(h, 0x5CA1E, 0);
    let steps = u64::from((SCALE_MAX_PCT - SCALE_MIN_PCT) / SCALE_STEP_PCT) + 1;
    let pct = SCALE_MIN_PCT + SCALE_STEP_PCT * u8::try_from(h % steps).unwrap_or(0);
    (pct, (h >> 32) & 1 == 1)
}

/// The family a query placement draws from, picked by seed and index
/// among its matches' families in order of first appearance. Without
/// takes this is the pick among the matches themselves.
fn pick_family<'a>(all: &[&'a Asset], seed: u64, index: usize) -> Option<&'a str> {
    let mut families: Vec<&str> = Vec::new();
    for a in all {
        let f = family_base(&a.id);
        if !families.contains(&f) {
            families.push(f);
        }
    }
    let n = families.len() as u64;
    if n == 0 {
        return None;
    }
    let pick = hash2(seed ^ 0x9E50, i64::try_from(index).unwrap_or(0), 0) % n;
    families.get(usize::try_from(pick).ok()?).copied()
}

/// The placement's own rotation and mirror, or for a [`ROT_FREE`] asset
/// left at rotation 0 unmirrored, a hashed one among those the asset
/// allows. Only assets anchored at their centre are turned, so the
/// footprint stays where the layout put it; non-square ones only by half
/// turns, so it keeps its shape.
fn turn(a: &Asset, p: &Placement, h: u64) -> (u16, bool) {
    let free = a.tags.free.iter().any(|t| t == ROT_FREE);
    let centred = a.anchor_or_centre() == a.footprint_centre();
    if !free || !centred || p.rotation != 0 || p.mirror {
        return (p.rotation, p.mirror);
    }
    let square = a.footprint.w == a.footprint.h;
    let turns: Vec<u16> = a
        .rotations
        .iter()
        .copied()
        .filter(|r| square || r % 180 == 0)
        .collect();
    let h = hash2(h, 0x207, 0);
    let rotation = turns
        .get(usize::try_from(h % turns.len().max(1) as u64).unwrap_or(0))
        .copied()
        .unwrap_or(0);
    (rotation, a.mirror && (h >> 32) & 1 == 1)
}
