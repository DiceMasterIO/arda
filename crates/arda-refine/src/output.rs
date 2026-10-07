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
/// Trunks needed for a canopy to block sight.
const CANOPY_BLOCKS: usize = 4;
/// Trunks for a thinner canopy that lightly obscures (logic/09
/// §rules-sidecar "moderate foliage"; count assumed, tunable).
const CANOPY_LIGHT: usize = 2;

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

fn floor_i(v: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    let i = v.floor() as i64;
    i
}

/// Rules for every square of `b`; `items` includes the halo so canopy
/// counts near the edge match the neighbour's.
#[must_use]
pub fn square_rules(b: &Block, items: &[Item], illegal: &BTreeSet<usize>) -> Vec<SquareRules> {
    let mut out = Vec::with_capacity(SIDE * SIDE);
    let mut cover = vec![CoverRule::None; SIDE * SIDE];
    let mut hard = vec![false; SIDE * SIDE];
    let mut wall = vec![false; SIDE * SIDE];
    for it in items {
        // An outcrop fills the 3 × 3 squares around its anchor.
        let r = if it.kind == Kind::Outcrop { 1 } else { 0 };
        for dy in -r..=r {
            for dx in -r..=r {
                let (i, j) = (
                    floor_i(it.x) + dx - b.origin.0,
                    floor_i(it.y) + dy - b.origin.1,
                );
                let (Ok(i), Ok(j)) = (usize::try_from(i), usize::try_from(j)) else {
                    continue;
                };
                if i >= SIDE || j >= SIDE {
                    continue;
                }
                let (c, d) = rules_of(it);
                cover[j * SIDE + i] = cover[j * SIDE + i].max(c);
                hard[j * SIDE + i] |= d;
                wall[j * SIDE + i] |= r > 0;
            }
        }
    }
    let trunks: Vec<&Item> = items
        .iter()
        .filter(|it| it.kind == Kind::TreeLarge)
        .collect();
    for j in 0..SIDE {
        for i in 0..SIDE {
            let k = j * SIDE + i;
            let (cx, cy) = (
                b.origin.0 as f64 + i as f64 + 0.5,
                b.origin.1 as f64 + j as f64 + 0.5,
            );
            let canopy = trunks
                .iter()
                .filter(|t| (t.x - cx) * (t.x - cx) + (t.y - cy) * (t.y - cy) < CANOPY_R * CANOPY_R)
                .count();
            let depth = b.depth_ft[k];
            out.push(SquareRules {
                difficult: hard[k] || difficult_ground(b.ground[k], depth),
                water_depth_ft: depth,
                cover: cover[k],
                blocks_sight: canopy >= CANOPY_BLOCKS && depth == 0,
                lightly_obscured: ((CANOPY_LIGHT..CANOPY_BLOCKS).contains(&canopy) && depth == 0)
                    || b.ground[k] == "reed_bed",
                blocks_movement: wall[k],
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
        blocks_movement: false,
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
            if it.x < x0 as f64 || it.y < y0 as f64 || it.x >= x1 as f64 || it.y >= y1 as f64 {
                continue;
            }
            placements.push(Placement {
                asset: AssetRef::Id(it.asset.to_string()),
                x: fixed64(it.x - x0 as f64),
                y: fixed64(it.y - y0 as f64),
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
