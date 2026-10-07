//! Tactical blocks: any window of 5-ft squares cut from a town plan as an
//! `arda_tactical::TacticalLayout` (goals 42–46, 48, 64).
//!
//! The layout uses the canonical vocabulary keys; [`fallback::resolve`]
//! maps it onto a concrete library. A sidecar lists the plan buildings in
//! the window by their plan ids (goal 45) and the per-square rules.
//!
//! Seams: every square, edge, prop and light is a pure function of global
//! square coordinates and plan objects. Walls on a window's border edge
//! appear in both neighbours; props overlapping the border appear in both,
//! offset, and the compositor clips them, so adjacent blocks line up.

pub mod exterior;
pub mod fallback;
pub mod frame;
pub mod ground;
pub mod interior;
pub mod kits;
pub mod wfc;
pub mod yard;

use crate::error::TownError;
use crate::function::BuildingFunction;
use crate::num::{clamp_u32, f32_of};
use crate::plan::grid::SquareRect;
use crate::plan::{BuildingId, Door, TownPlan};
use crate::site::SettlementId;
use arda_tactical::catalog::{AssetClass, WallRole};
use arda_tactical::layout::{
    AssetRef, EdgeAxis, LightSource, Placement, Square, TacticalLayout, WallSegment,
};
use frame::Want;
use interior::{GLight, GProp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use wfc::{Relaxed, TownFill};

/// Largest window edge, squares.
pub const MAX_WINDOW: i64 = 1024;

/// A window of global squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// First column (global square).
    pub x: i64,
    /// First row (global square).
    pub y: i64,
    /// Width in squares.
    pub w: i64,
    /// Height in squares.
    pub h: i64,
}

/// A plan building as seen by one block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockBuilding {
    /// Plan id (the same object as in the plan and `BuildingSpec`).
    pub id: BuildingId,
    /// Function.
    pub function: BuildingFunction,
    /// Whole footprint, global squares.
    pub rect: SquareRect,
    /// The part inside this block, local squares `[x0, y0, x1, y1)`.
    pub local: [i64; 4],
    /// Doors, global squares.
    pub doors: Vec<Door>,
    /// Rooms `(kind, global rectangle)`.
    pub rooms: Vec<(String, SquareRect)>,
}

/// Per-square rules, mirroring `arda-scene` `RulesSidecar` format 2 names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquareRules {
    /// Difficult terrain (shallow water, mud).
    pub difficult: bool,
    /// Water depth, feet.
    pub water_depth_ft: u8,
    /// Cover level (`none` here; walls are edges).
    pub cover: Cover,
    /// Blocks sight.
    pub blocks_sight: bool,
    /// Blocks movement.
    pub blocks_movement: bool,
    /// A deck over water (bridge or dock).
    pub deck: bool,
}

/// SRD cover levels (I16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// No cover.
    None,
    /// Half cover.
    Half,
    /// Three-quarters cover.
    ThreeQuarters,
    /// Total cover.
    Total,
}

/// A tactical block and its sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TownBlock {
    /// Settlement id.
    pub site: SettlementId,
    /// The window, global squares.
    pub window: Window,
    /// World-square origin of layout square (0, 0) (convention I10).
    pub origin: [i64; 2],
    /// The layout with canonical vocabulary keys.
    pub layout: TacticalLayout,
    /// Buildings touching the window.
    pub buildings: Vec<BlockBuilding>,
    /// Per-square rules, row-major.
    pub rules: Vec<SquareRules>,
    /// Row-major: whether the town reserves the square (every plan kind
    /// but open country and water; logic/09 §reservations precedence 1).
    pub owned: Vec<bool>,
    /// For each light, the placement that emits it (dropped when the
    /// resolved asset carries its own light).
    #[serde(skip)]
    pub light_owner: Vec<Option<usize>>,
    /// WFC problems in or around the window that used the relaxed fill
    /// (goal 47: marked for review).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relaxed: Vec<Relaxed>,
}

fn priority(r: WallRole) -> u8 {
    match r {
        WallRole::Door | WallRole::Gate => 3,
        WallRole::Run => 2,
        _ => 1,
    }
}

fn asset(w: &Want) -> AssetRef {
    match w {
        Want::Id(id) => AssetRef::Id((*id).to_string()),
        Want::Query(tags) => AssetRef::Query {
            class: Some(AssetClass::Prop),
            tags: tags.clone(),
        },
    }
}

type Walls = BTreeMap<(i64, i64, EdgeAxis), (WallRole, &'static str, u64)>;

/// Cuts a window out of the plan with the default fill ([`TownFill::Wfc`]).
///
/// # Errors
/// [`TownError::Window`] for an empty or oversized window.
pub fn generate(plan: &TownPlan, win: Window) -> Result<TownBlock, TownError> {
    generate_with(plan, win, TownFill::default())
}

/// Cuts a window out of the plan, filling the town fabric with `fill`.
///
/// # Errors
/// [`TownError::Window`] for an empty or oversized window.
pub fn generate_with(plan: &TownPlan, win: Window, fill: TownFill) -> Result<TownBlock, TownError> {
    if win.w < 1 || win.h < 1 || win.w > MAX_WINDOW || win.h > MAX_WINDOW {
        return Err(TownError::Window(format!(
            "{}x{} is outside 1..={MAX_WINDOW}",
            win.w, win.h
        )));
    }
    let (x0, y0, x1, y1) = (win.x, win.y, win.x + win.w, win.y + win.h);
    let mut grounds = Vec::with_capacity(usize::try_from(win.w * win.h).unwrap_or(0));
    let mut owned = Vec::with_capacity(grounds.capacity());
    let poor = ground::poor(plan);
    for y in y0..y1 {
        for x in x0..x1 {
            grounds.push(ground::at(plan, poor, x, y));
            owned.push(!matches!(
                ground::kind(plan, x, y),
                crate::plan::grid::Kind::Open | crate::plan::grid::Kind::Water
            ));
        }
    }
    let lin = |x: i64, y: i64| usize::try_from((y - y0) * win.w + (x - x0)).ok();
    let mut walls: Walls = BTreeMap::new();
    for (k, v) in ground::edges(plan, x0, y0, x1, y1) {
        walls.insert(k, (v.0, v.1, 0));
    }
    let mut props: Vec<GProp> = Vec::new();
    let mut lights: Vec<GLight> = Vec::new();
    let mut relaxed = Vec::new();
    if fill == TownFill::Wfc {
        let view = SquareRect { x0, y0, x1, y1 };
        let site = wfc::outdoor::Site::of(plan);
        for (cx, cy) in wfc::outdoor::chunks_near(x0, y0, x1, y1) {
            let ch = wfc::outdoor::chunk(plan, &site, cx, cy);
            if !ch.any {
                continue;
            }
            let c = wfc::outdoor::CHUNK;
            let area = SquareRect {
                x0: cx * c,
                y0: cy * c,
                x1: cx * c + c,
                y1: cy * c + c,
            };
            if ch.relaxed && area.overlaps(&view) {
                relaxed.push(Relaxed::Outdoor { cx, cy });
            }
            for &((x, y), key) in &ch.ground {
                if let Some(i) = view.contains(x, y).then(|| lin(x, y)).flatten() {
                    grounds[i].key = key;
                }
            }
            for &(k, (role, kit)) in &ch.walls {
                if walls
                    .get(&k)
                    .is_some_and(|w| w.0 == WallRole::Run && w.2 == 0)
                {
                    walls.insert(k, (role, kit, 0));
                }
            }
            let base = props.len();
            props.extend(ch.props.iter().cloned());
            lights.extend(ch.lights.iter().map(|l| GLight {
                owner: l.owner.map(|o| o + base),
                ..*l
            }));
        }
    }
    let mut buildings = Vec::new();
    let view = SquareRect { x0, y0, x1, y1 };
    for b in plan
        .buildings
        .iter()
        .filter(|b| b.rect.grown(1).overlaps(&view))
    {
        let int = match fill {
            TownFill::Rules => interior::build(plan, b),
            TownFill::Wfc => {
                let built = wfc::indoor::build(plan, b);
                if built.relaxed && b.rect.overlaps(&view) {
                    relaxed.push(Relaxed::Interior { building: b.id.0 });
                }
                built.interior
            }
        };
        let floor_ft = b
            .doors
            .first()
            .map_or(ground::elevation(plan, b.rect.x0, b.rect.y0), |d| {
                ground::elevation(plan, d.x, d.y)
            });
        for &((x, y), key) in &int.floors {
            let Some(i) = view.contains(x, y).then(|| lin(x, y)).flatten() else {
                continue;
            };
            if b.function.walled() {
                grounds[i] = ground::Ground {
                    key,
                    elevation_ft: floor_ft,
                    water_ft: 0,
                    deck: false,
                };
            } else if b.function == BuildingFunction::Dock {
                grounds[i].deck = true;
            }
        }
        for &(k, (role, kit)) in &int.walls {
            let (x, y, axis) = k;
            let inside = match axis {
                EdgeAxis::Horizontal => x >= x0 && x < x1 && y >= y0 && y <= y1,
                EdgeAxis::Vertical => x >= x0 && x <= x1 && y >= y0 && y < y1,
            };
            if !inside {
                continue;
            }
            // Shared party walls: the stronger piece wins, regardless of
            // which building was visited first (doors over runs over windows).
            // The curtain wall (owner 0) always wins.
            let keep = walls.get(&k).is_some_and(|&(r, _, owner)| {
                owner == 0 || (owner != b.id.0 && (priority(r), owner) >= (priority(role), b.id.0))
            });
            if !keep {
                walls.insert(k, (role, kit, b.id.0));
            }
        }
        let base = props.len();
        props.extend(int.props.iter().cloned());
        lights.extend(int.lights.iter().map(|l| GLight {
            owner: l.owner.map(|o| o + base),
            ..*l
        }));
        if b.rect.overlaps(&view) {
            let r = b.rect;
            buildings.push(BlockBuilding {
                id: b.id,
                function: b.function,
                rect: r,
                local: [
                    r.x0.max(x0) - x0,
                    r.y0.max(y0) - y0,
                    r.x1.min(x1) - x0,
                    r.y1.min(y1) - y0,
                ],
                doors: b.doors.clone(),
                rooms: int
                    .rooms
                    .iter()
                    .map(|(k, r)| ((*k).to_string(), *r))
                    .collect(),
            });
        }
    }
    // Fences and hedges never replace a building's or the curtain's wall.
    for (k, v) in ground::fences(plan, x0, y0, x1, y1) {
        walls.entry(k).or_insert((v.0, v.1, u64::MAX));
    }
    let (ext_props, ext_lights) = match fill {
        TownFill::Rules => exterior::dress(plan, x0, y0, x1, y1),
        TownFill::Wfc => exterior::fixed(plan, x0, y0, x1, y1),
    };
    let base = props.len();
    props.extend(ext_props);
    lights.extend(ext_lights.into_iter().map(|l| GLight {
        owner: l.owner.map(|o| o + base),
        ..l
    }));
    let mut blk = assemble(plan, win, &grounds, &walls, &props, &lights, buildings);
    blk.owned = owned;
    relaxed.sort();
    relaxed.dedup();
    blk.relaxed = relaxed;
    Ok(blk)
}

fn assemble(
    plan: &TownPlan,
    win: Window,
    grounds: &[ground::Ground],
    walls: &Walls,
    props: &[GProp],
    lights: &[GLight],
    buildings: Vec<BlockBuilding>,
) -> TownBlock {
    #[allow(clippy::cast_precision_loss)]
    let (fx, fy, fw, fh) = (win.x as f64, win.y as f64, win.w as f64, win.h as f64);
    let name = format!("{}-{}-{}", plan.name.to_lowercase(), win.x, win.y);
    let mut layout = TacticalLayout::new(&name, clamp_u32(win.w), clamp_u32(win.h), "grass");
    // Convention I10: the layout carries its world origin in squares.
    layout.origin = Some([win.x, win.y]);
    layout.squares = grounds
        .iter()
        .map(|g| Square {
            ground: g.key.to_string(),
            elevation_ft: g.elevation_ft,
            water_depth_ft: g.water_ft,
        })
        .collect();
    layout.walls = walls
        .iter()
        .map(|(&(x, y, axis), &(kind, kit, _))| WallSegment {
            x: clamp_u32(x - win.x),
            y: clamp_u32(y - win.y),
            axis,
            kind,
            kit: kit.to_string(),
            tags: Vec::new(),
        })
        .collect();
    let mut remap = vec![None; props.len()];
    for (i, p) in props.iter().enumerate() {
        let hit =
            p.extent[0] < fx + fw && p.extent[2] > fx && p.extent[1] < fy + fh && p.extent[3] > fy;
        if hit {
            remap[i] = Some(layout.placements.len());
            layout.placements.push(Placement {
                asset: asset(&p.want),
                x: f32_of(p.x - fx),
                y: f32_of(p.y - fy),
                rotation: p.rot,
                mirror: false,
            });
        }
    }
    let mut light_owner = Vec::new();
    for l in lights {
        let reach = f64::from(l.radius_ft) / 5.0;
        let near =
            l.x > fx - reach && l.x < fx + fw + reach && l.y > fy - reach && l.y < fy + fh + reach;
        if near {
            layout.lights.push(LightSource {
                x: f32_of(l.x - fx),
                y: f32_of(l.y - fy),
                radius_ft: l.radius_ft,
                colour: l.colour,
            });
            light_owner.push(l.owner.and_then(|o| remap.get(o).copied().flatten()));
        }
    }
    let rules = grounds
        .iter()
        .map(|g| SquareRules {
            difficult: (g.water_ft > 0 && g.water_ft < 5 && !g.deck)
                || (g.key == "mud" && g.water_ft == 0),
            water_depth_ft: if g.deck { 0 } else { g.water_ft },
            cover: Cover::None,
            blocks_sight: false,
            blocks_movement: false,
            deck: g.deck,
        })
        .collect();
    TownBlock {
        site: plan.site,
        window: win,
        origin: [win.x, win.y],
        layout,
        buildings,
        rules,
        owned: Vec::new(),
        light_owner,
        relaxed: Vec::new(),
    }
}

impl From<Cover> for arda_scene::CoverLevel {
    fn from(c: Cover) -> Self {
        match c {
            Cover::None => Self::None,
            Cover::Half => Self::Half,
            Cover::ThreeQuarters => Self::ThreeQuarters,
            Cover::Total => Self::Total,
        }
    }
}

impl TownBlock {
    /// The `arda-scene` `RulesSidecar` format 2 of this block (adapter A8,
    /// I9): owned squares state their rules in full, with the water depth
    /// under a deck kept and `deck` set; a square inside a plan building
    /// carries its id as `ext.building` (a string, I5). Other squares state
    /// nothing, so the layers beneath keep theirs.
    #[must_use]
    pub fn to_rules(&self) -> arda_scene::RulesSidecar {
        let (w, h) = (self.layout.width, self.layout.height);
        let mut r = arda_scene::RulesSidecar::empty(w, h);
        for (i, cell) in r.squares.iter_mut().enumerate() {
            if !self.owned.get(i).copied().unwrap_or(false) {
                continue;
            }
            let (Some(sr), Some(sq)) = (self.rules.get(i), self.layout.squares.get(i)) else {
                continue;
            };
            cell.difficult = Some(sr.difficult);
            cell.water_depth_ft = Some(sq.water_depth_ft);
            cell.cover = Some(sr.cover.into());
            cell.blocks_sight = Some(sr.blocks_sight);
            cell.blocks_movement = Some(sr.blocks_movement);
            cell.deck = Some(sr.deck);
        }
        let wi = i64::from(w);
        for b in &self.buildings {
            let [x0, y0, x1, y1] = b.local;
            for y in y0.max(0)..y1.min(i64::from(h)) {
                for x in x0.max(0)..x1.min(wi) {
                    let Some(cell) = usize::try_from(y * wi + x)
                        .ok()
                        .and_then(|i| r.squares.get_mut(i))
                    else {
                        continue;
                    };
                    cell.set_ext("building", serde_json::Value::from(b.id.0.to_string()));
                }
            }
        }
        r
    }

    /// Serialises the block and sidecar as pretty JSON.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
