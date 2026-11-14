//! Converting refined blocks into the `TacticalLayout` schema and the rules
//! sidecar, for one block or a window across blocks.

use crate::block::{difficult_ground, item_rules, Block, SIDE};
use crate::grid::span;
use crate::layout::{AssetRef, Placement, Square, TacticalLayout};
use crate::rules::{
    BlockMeta, CoverRule, MapMeta, PlacementMeta, RulesSidecar, SquareRules, META_FORMAT,
    SIDECAR_FORMAT_VERSION,
};
use crate::scatter::{Item, Kind};
use std::collections::BTreeSet;

/// Trunks within this radius of a square centre count toward its canopy.
const CANOPY_R: f64 = 2.4;
/// Trunks for a canopy that lightly obscures (logic/09 §rules-sidecar
/// "moderate foliage"; count assumed, tunable). A denser canopy obscures no
/// more: a closed wood is foliage overhead, not a thicket.
const CANOPY_LIGHT: usize = 2;
/// Woody understory within this radius of a square centre counts toward a
/// thicket (logic/09 §rules-sidecar "thicket").
const THICKET_R: f64 = 2.0;
/// Small trees and woody shrubs that make a thicket, which heavily
/// obscures (SRD 5.1 "dense foliage"; count tuned on the MICRO-42 kit
/// world, 2026-11-14).
const THICKET_MIN: usize = 4;

/// Whether an item is foliage the sight rules count: a large trunk (canopy)
/// or woody understory (a small or stunted tree, a shrub; not a fern).
fn foliage(it: &Item) -> bool {
    match it.kind {
        Kind::TreeLarge | Kind::TreeSmall => true,
        Kind::Undergrowth => it.tag != "undergrowth:fern",
        _ => false,
    }
}

/// `(blocks_sight, lightly_obscured)` of the square centred on `at`
/// (logic/09 §rules-sidecar): a thicket, at least [`THICKET_MIN`] woody
/// understory items within [`THICKET_R`], blocks sight; a canopy of at
/// least [`CANOPY_LIGHT`] trunks within [`CANOPY_R`], or reeds, lightly
/// obscures. Water is never a thicket or a canopy floor. `items` may hold
/// any items; only [`foliage`] counts.
fn sight(items: &[Item], at: (f64, f64), depth: u8, ground: &str) -> (bool, bool) {
    let near = |it: &Item, r: f64| {
        let (dx, dy) = (it.x - at.0, it.y - at.1);
        dx * dx + dy * dy < r * r
    };
    let mut canopy = 0;
    let mut thicket = 0;
    for it in items.iter().filter(|it| foliage(it)) {
        if it.kind == Kind::TreeLarge {
            canopy += usize::from(near(it, CANOPY_R));
        } else {
            thicket += usize::from(near(it, THICKET_R));
        }
    }
    let dry = depth == 0;
    let heavy = dry && thicket >= THICKET_MIN;
    let light = !heavy && ((dry && canopy >= CANOPY_LIGHT) || ground == "reed_bed");
    (heavy, light)
}

/// Cover and difficulty of one item, refined by its tag.
#[must_use]
pub fn rules_of(it: &Item) -> (CoverRule, bool) {
    let (cover, difficult) = item_rules(it.kind);
    let cover = match it.tag {
        "rock:small" | "rock:stones" | "rock:scree" => CoverRule::None,
        _ => cover,
    };
    (cover, difficult || it.tag == "rock:scree")
}

/// Rules for every square of `b`; `items` includes the halo so canopy
/// counts near the edge match the neighbour's. Ground, water, canopy and
/// the cover of the item anchored on each square; an item's blocking and
/// difficulty are its catalogue entry's (logic/12 §scene-sidecar).
#[must_use]
pub fn square_rules(b: &Block, items: &[Item], illegal: &BTreeSet<usize>) -> Vec<SquareRules> {
    let mut out = Vec::with_capacity(SIDE * SIDE);
    let mut cover = vec![CoverRule::None; SIDE * SIDE];
    for it in items {
        #[allow(clippy::cast_possible_truncation)]
        let (i, j) = (
            it.x.floor() as i64 - b.origin.0,
            it.y.floor() as i64 - b.origin.1,
        );
        let (Ok(i), Ok(j)) = (usize::try_from(i), usize::try_from(j)) else {
            continue;
        };
        if i < SIDE && j < SIDE {
            cover[j * SIDE + i] = cover[j * SIDE + i].max(rules_of(it).0);
        }
    }
    let leaves: Vec<Item> = items.iter().filter(|it| foliage(it)).cloned().collect();
    for j in 0..SIDE {
        for i in 0..SIDE {
            let k = j * SIDE + i;
            let (cx, cy) = (
                b.origin.0 as f64 + i as f64 + 0.5,
                b.origin.1 as f64 + j as f64 + 0.5,
            );
            let depth = b.depth_ft[k];
            let (blocks_sight, lightly_obscured) = sight(&leaves, (cx, cy), depth, b.ground[k]);
            out.push(SquareRules {
                difficult: difficult_ground(b.ground[k], depth),
                water_depth_ft: depth,
                cover: cover[k],
                blocks_sight,
                lightly_obscured,
                review: b.relaxed && illegal.contains(&k),
            });
        }
    }
    out
}

/// A layout plus its rules sidecar and metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Map {
    /// The layout in the `arda-tactical` schema.
    pub layout: TacticalLayout,
    /// The SRD rules sidecar (`arda-scene` field names).
    pub rules: RulesSidecar,
    /// Origin, review flags, block records and placement tags.
    pub meta: MapMeta,
}

impl Map {
    /// Pretty JSON of the layout.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn layout_json(&self) -> Result<String, crate::RefineError> {
        Ok(serde_json::to_string_pretty(&self.layout)?)
    }

    /// Pretty JSON of the rules sidecar.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn rules_json(&self) -> Result<String, crate::RefineError> {
        Ok(serde_json::to_string_pretty(&self.rules)?)
    }

    /// Pretty JSON of the metadata.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn meta_json(&self) -> Result<String, crate::RefineError> {
        Ok(serde_json::to_string_pretty(&self.meta)?)
    }
}

/// Builds the map for the global square window `[x0, x0 + w) × [y0, y0 + h)`
/// from blocks that cover it.
#[must_use]
pub fn assemble<B: std::borrow::Borrow<Block>>(
    name: &str,
    blocks: &[B],
    x0: i64,
    y0: i64,
    w: u32,
    h: u32,
) -> Map {
    let blocks: Vec<&Block> = blocks.iter().map(std::borrow::Borrow::borrow).collect();
    let (wu, hu) = (w as usize, h as usize);
    let mut squares = vec![
        Square {
            ground: "grass".into(),
            elevation_ft: 0,
            water_depth_ft: 0,
            dryness: 0,
        };
        wu * hu
    ];
    let blank = SquareRules {
        difficult: false,
        water_depth_ft: 0,
        cover: CoverRule::None,
        blocks_sight: false,
        lightly_obscured: false,
        review: false,
    };
    let mut rules = vec![blank; wu * hu];
    let mut placements = Vec::new();
    let mut meta = Vec::new();
    let (x1, y1) = (x0 + i64::from(w), y0 + i64::from(h));
    for &b in &blocks {
        for j in 0..SIDE {
            for i in 0..SIDE {
                let (gx, gy) = (b.origin.0 + span(i), b.origin.1 + span(j));
                if gx < x0 || gy < y0 || gx >= x1 || gy >= y1 {
                    continue;
                }
                let (Ok(ox), Ok(oy)) = (usize::try_from(gx - x0), usize::try_from(gy - y0)) else {
                    continue;
                };
                let k = j * SIDE + i;
                squares[oy * wu + ox] = Square {
                    ground: b.ground[k].to_string(),
                    elevation_ft: b.elevation_ft[k],
                    water_depth_ft: b.depth_ft[k],
                    dryness: b.dryness[k],
                };
                rules[oy * wu + ox] = b.rules[k];
            }
        }
        for it in &b.items {
            // Snap in world squares first, so whether an item anchored on
            // a window's border belongs to it is the same for every window
            // (review 3.5-8: a tree at y = 79679.99999 fell in one window
            // and not the next).
            let (gx, gy) = (snap64(it.x), snap64(it.y));
            if gx < x0 as f64 || gy < y0 as f64 || gx >= x1 as f64 || gy >= y1 as f64 {
                continue;
            }
            placements.push(Placement {
                asset: AssetRef::Id(it.asset.to_string()),
                x: fixed64(gx - x0 as f64),
                y: fixed64(gy - y0 as f64),
                rotation: it.rotation,
                mirror: it.mirror,
            });
            let (cover, difficult) = rules_of(it);
            meta.push(PlacementMeta {
                tag: it.tag.to_string(),
                cover,
                difficult,
            });
        }
    }
    let review = rules
        .iter()
        .enumerate()
        .filter(|(_, r)| r.review)
        .filter_map(|(i, _)| u32::try_from(i).ok())
        .collect();
    Map {
        layout: TacticalLayout {
            name: name.to_string(),
            width: w,
            height: h,
            squares,
            walls: Vec::new(),
            placements,
            lights: Vec::new(),
            origin: Some([x0, y0]),
        },
        rules: RulesSidecar {
            format_version: SIDECAR_FORMAT_VERSION,
            width: w,
            height: h,
            squares: rules.iter().map(SquareRules::cell).collect(),
            edges: Vec::new(),
        },
        meta: MapMeta {
            format: META_FORMAT.to_string(),
            name: name.to_string(),
            width: w,
            height: h,
            relaxed: blocks.iter().any(|b| b.relaxed),
            review,
            blocks: blocks
                .iter()
                .map(|b| BlockMeta {
                    cell: [b.cell.x, b.cell.y],
                    attempts: b.attempts,
                    repairs: b.repairs,
                    relaxed: b.relaxed,
                })
                .collect(),
            placements: meta,
        },
    }
}

/// Snaps a world coordinate, in squares, to 1/64 square in `f64`.
#[must_use]
pub fn snap64(v: f64) -> f64 {
    (v * 64.0).round() / 64.0
}

/// Snaps a coordinate to 1/64 square, a fixed-point value that `f32` and
/// JSON represent exactly.
#[must_use]
pub fn fixed64(v: f64) -> f32 {
    let q = (v * 64.0).round().clamp(-8_388_608.0, 8_388_607.0);
    #[allow(clippy::cast_possible_truncation)]
    let q = q as i32;
    #[allow(clippy::cast_precision_loss)]
    let f = q as f32 / 64.0;
    f
}

#[cfg(test)]
mod tests {
    use super::sight;
    use crate::scatter::{Item, Kind};

    fn item(x: f64, y: f64, kind: Kind, tag: &'static str) -> Item {
        Item {
            x,
            y,
            kind,
            asset: "veg.test",
            tag,
            rotation: 0,
            mirror: false,
        }
    }

    /// Items at `n` points on a ring of radius `r` around (10.5, 10.5).
    fn ring(n: usize, r: f64, kind: Kind, tag: &'static str) -> Vec<Item> {
        let pts = [
            (1.0, 0.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (0.7, 0.7),
            (-0.7, -0.7),
        ];
        pts.iter()
            .take(n)
            .map(|(dx, dy)| item(10.5 + dx * r, 10.5 + dy * r, kind, tag))
            .collect()
    }

    const AT: (f64, f64) = (10.5, 10.5);

    // logic/09 §rules-sidecar: a closed canopy is foliage overhead, not a
    // thicket; a creature under it is lightly obscured, never blind.
    #[test]
    fn a_closed_canopy_lightly_obscures() {
        let trunks = ring(6, 1.5, Kind::TreeLarge, "tree:broadleaf:large");
        assert_eq!(sight(&trunks, AT, 0, "forest_floor"), (false, true));
    }

    #[test]
    fn a_thicket_of_woody_understory_heavily_obscures() {
        let mut items = ring(2, 1.5, Kind::TreeSmall, "tree:broadleaf:small");
        items.extend(
            ring(4, 1.8, Kind::Undergrowth, "undergrowth:bush")
                .into_iter()
                .skip(2),
        );
        assert_eq!(sight(&items, AT, 0, "forest_floor"), (true, false));
    }

    #[test]
    fn three_shrubs_or_ferns_are_no_thicket() {
        let shrubs = ring(3, 1.5, Kind::Undergrowth, "undergrowth:bush");
        assert_eq!(sight(&shrubs, AT, 0, "scrub"), (false, false));
        let ferns = ring(6, 1.5, Kind::Undergrowth, "undergrowth:fern");
        assert_eq!(sight(&ferns, AT, 0, "forest_floor"), (false, false));
    }

    #[test]
    fn understory_beyond_two_squares_does_not_count() {
        let far = ring(6, 2.1, Kind::TreeSmall, "tree:conifer:small");
        assert_eq!(sight(&far, AT, 0, "forest_floor"), (false, false));
    }

    #[test]
    fn water_is_never_a_thicket_and_reeds_lightly_obscure() {
        let items = ring(6, 1.5, Kind::TreeSmall, "tree:broadleaf:small");
        assert_eq!(sight(&items, AT, 2, "forest_floor"), (false, false));
        assert_eq!(sight(&[], AT, 1, "reed_bed"), (false, true));
    }

    #[test]
    fn open_ground_is_clear() {
        let lone = ring(1, 1.0, Kind::TreeLarge, "tree:broadleaf:large");
        assert_eq!(sight(&lone, AT, 0, "grass"), (false, false));
    }
}
