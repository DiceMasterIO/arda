//! Rasterising a [`Plan`] onto the square lattice. Work happens on a grid
//! a few squares larger than the window (the apron), from world functions
//! only, so edge decisions near the border agree with the neighbour window.

mod house;
mod roads;
mod structures;

use crate::input::{m_to_ft, step5, RoadClass, Terrain, SQUARE_M, SWIM_FT};
use crate::plan::{square_centre, Plan, Window};
use crate::sidecar::{Cover, EdgeRole, EdgeRule, Feature, Sidecar, SquareRules, SIDECAR_VERSION};
use arda_tactical::layout::{AssetRef, EdgeAxis, LightSource, Placement, WallSegment};
use arda_tactical::noise::hash2;
use arda_tactical::{TacticalLayout, WallRole};
use std::collections::BTreeMap;

/// Squares computed beyond each window edge.
pub const APRON: i64 = 3;
/// Placements anchored this many squares outside the window are kept, so
/// sprites overlapping the border are drawn on both sides.
pub const PROP_MARGIN: f64 = 3.0;

/// One square of the working grid.
#[derive(Debug, Clone, Default)]
pub struct Cell {
    /// Ground override.
    pub ground: Option<&'static str>,
    /// Water depth override, feet.
    pub water_ft: Option<u8>,
    /// Elevation override, feet.
    pub elev_ft: Option<i16>,
    /// Terrain height, feet.
    pub terrain_ft: i16,
    /// Channel water here (before crossings).
    pub water: bool,
    /// Role.
    pub feature: Feature,
    /// Road class owning the square.
    pub class: Option<RoadClass>,
    /// Band priority and class of the owner, for overlaps.
    pub rank: (u8, u8),
    /// Deck level when the square is decked.
    pub deck_ft: Option<i16>,
    /// Difficult terrain.
    pub difficult: bool,
}

impl Cell {
    /// Elevation after overrides.
    #[must_use]
    pub fn elevation(&self) -> i16 {
        self.elev_ft.unwrap_or(self.terrain_ft)
    }
}

/// A wall on the global lattice.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wall {
    /// Global column.
    pub gx: i64,
    /// Global row.
    pub gy: i64,
    /// Edge.
    pub axis: EdgeAxis,
    /// Piece role.
    pub kind: WallRole,
    /// Kit.
    pub kit: &'static str,
    /// Rules role.
    pub role: EdgeRole,
}

/// A prop on the global lattice, anchored in squares.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    /// Global x in squares.
    pub x: f64,
    /// Global y in squares.
    pub y: f64,
    /// Asset id.
    pub id: &'static str,
    /// Rotation, degrees.
    pub rotation: u16,
}

/// The working grid.
pub struct Grid {
    /// Global column of cell 0.
    pub gx0: i64,
    /// Global row of cell 0.
    pub gy0: i64,
    /// Width in cells.
    pub w: i64,
    /// Height in cells.
    pub h: i64,
    /// Row-major cells.
    pub cells: Vec<Cell>,
    /// Walls.
    pub walls: Vec<Wall>,
    /// Props.
    pub props: Vec<Prop>,
    /// Lights `(x, y, radius_ft)` in global squares.
    pub lights: Vec<(f64, f64, u16)>,
    /// Render seed.
    pub seed: u64,
}

impl Grid {
    fn new(win: Window, terrain: &dyn Terrain, seed: u64) -> Self {
        let (w, h) = (
            i64::from(win.width) + 2 * APRON,
            i64::from(win.height) + 2 * APRON,
        );
        let (gx0, gy0) = (win.gx0 - APRON, win.gy0 - APRON);
        let mut cells = Vec::with_capacity(usize::try_from(w * h).unwrap_or(0));
        for gy in gy0..gy0 + h {
            for gx in gx0..gx0 + w {
                let p = square_centre(gx, gy);
                cells.push(Cell {
                    terrain_ft: m_to_ft(terrain.height_m(p[0], p[1])),
                    ..Cell::default()
                });
            }
        }
        Self {
            gx0,
            gy0,
            w,
            h,
            cells,
            walls: Vec::new(),
            props: Vec::new(),
            lights: Vec::new(),
            seed,
        }
    }

    fn index(&self, gx: i64, gy: i64) -> Option<usize> {
        let (x, y) = (gx - self.gx0, gy - self.gy0);
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        usize::try_from(y * self.w + x).ok()
    }

    /// Cell at global `(gx, gy)`.
    #[must_use]
    pub fn get(&self, gx: i64, gy: i64) -> Option<&Cell> {
        self.index(gx, gy).and_then(|i| self.cells.get(i))
    }

    /// Mutable cell at global `(gx, gy)`.
    pub fn get_mut(&mut self, gx: i64, gy: i64) -> Option<&mut Cell> {
        self.index(gx, gy).and_then(|i| self.cells.get_mut(i))
    }

    /// Every global square of the grid, row-major.
    pub fn squares(&self) -> impl Iterator<Item = (i64, i64)> + '_ {
        (self.gy0..self.gy0 + self.h)
            .flat_map(move |gy| (self.gx0..self.gx0 + self.w).map(move |gx| (gx, gy)))
    }

    /// Deterministic unit hash of a global square and salt.
    #[must_use]
    pub fn hash(&self, salt: u64, a: i64, b: i64) -> f64 {
        #[allow(clippy::cast_precision_loss)] // 24 bits
        let v = (hash2(self.seed ^ salt, a, b) >> 40) as f64;
        v / f64::from(1u32 << 24)
    }

    /// Adds a prop.
    pub fn prop(&mut self, x: f64, y: f64, id: &'static str, rotation: u16) {
        self.props.push(Prop { x, y, id, rotation });
    }

    /// Adds a wall run.
    pub fn wall(
        &mut self,
        gx: i64,
        gy: i64,
        axis: EdgeAxis,
        kind: WallRole,
        kit: &'static str,
        role: EdgeRole,
    ) {
        self.walls.push(Wall {
            gx,
            gy,
            axis,
            kind,
            kit,
            role,
        });
    }
}

/// Marks the terrain's rasterised river water ([`Terrain::river_water`]):
/// the caller draws it, so only the flag is set; dry squares beside it are
/// banks.
fn mark_raster_water(g: &mut Grid, terrain: &dyn Terrain) {
    if !terrain.rivers_rasterised() {
        return;
    }
    let squares: Vec<(i64, i64)> = g.squares().collect();
    for &(gx, gy) in &squares {
        if terrain.river_water(gx, gy) {
            if let Some(c) = g.get_mut(gx, gy) {
                c.water = true;
            }
        }
    }
    for &(gx, gy) in &squares {
        let bank = g.get(gx, gy).is_some_and(|c| !c.water)
            && (-1..=1)
                .any(|dy| (-1..=1).any(|dx| g.get(gx + dx, gy + dy).is_some_and(|n| n.water)));
        if bank {
            if let Some(c) = g.get_mut(gx, gy) {
                c.feature = Feature::Bank;
            }
        }
    }
}

/// Paints channels: water squares with a rounded depth profile, mud banks.
/// Guide channels paint nothing (the terrain's raster is the water).
fn paint_channels(g: &mut Grid, plan: &Plan) {
    for (gx, gy) in g.squares().collect::<Vec<_>>() {
        let p = square_centre(gx, gy);
        for ch in plan.channels.iter().filter(|c| !c.guide) {
            let half = ch.width_m / 2.0;
            let Some(d) = ch.dist(p, half + SQUARE_M) else {
                continue;
            };
            let bank_mud = g.hash(0xBA4C, gx, gy) < 0.55;
            let Some(c) = g.get_mut(gx, gy) else { continue };
            if d <= half {
                let f = (1.0 - (d / half).powi(2)).max(0.0).sqrt();
                let ft = (ch.depth_m / 0.3048 * f).round().clamp(1.0, 60.0);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped
                let ft = ft as u8;
                c.water = true;
                c.water_ft = Some(c.water_ft.map_or(ft, |w| w.max(ft)));
                c.ground = Some("mud");
                c.feature = Feature::None;
            } else if !c.water {
                c.feature = Feature::Bank;
                if bank_mud {
                    c.ground = Some("mud");
                }
            }
        }
    }
}

/// Rasterises the plan and writes it into `layout` (window squares only),
/// returning the sidecar.
pub fn apply(
    layout: &mut TacticalLayout,
    plan: &Plan,
    terrain: &dyn Terrain,
    seed: u64,
) -> Sidecar {
    let mut g = Grid::new(plan.window, terrain, seed);
    mark_raster_water(&mut g, terrain);
    paint_channels(&mut g, plan);
    roads::paint(&mut g, plan, terrain);
    structures::paint(&mut g, plan, terrain);
    house::paint(&mut g, plan);
    roads::finish(&mut g, plan);
    emit(layout, &g, plan.window)
}

fn emit(layout: &mut TacticalLayout, g: &Grid, win: Window) -> Sidecar {
    let (w, h) = (layout.width, layout.height);
    let mut squares = Vec::with_capacity(w as usize * h as usize);
    for y in 0..h {
        for x in 0..w {
            let (gx, gy) = (win.gx0 + i64::from(x), win.gy0 + i64::from(y));
            let c = g.get(gx, gy).cloned().unwrap_or_default();
            let mut water = 0;
            if let Some(sq) = layout.square_mut(x, y) {
                if let Some(gr) = c.ground {
                    gr.clone_into(&mut sq.ground);
                }
                if let Some(wd) = c.water_ft {
                    sq.water_depth_ft = wd;
                }
                // Absolute feet in 5-ft steps (vocabulary I20).
                if let Some(e) = c.elev_ft {
                    sq.elevation_ft = step5(e);
                } else if c.water || c.feature != Feature::None {
                    sq.elevation_ft = step5(c.terrain_ft);
                }
                water = sq.water_depth_ft;
            }
            let deck = c.deck_ft.is_some();
            let wading = !deck && water > 0 && water < SWIM_FT;
            squares.push(SquareRules {
                difficult: c.difficult || wading,
                water_depth_ft: water,
                cover: Cover::None,
                blocks_sight: false,
                blocks_movement: false,
                deck,
                deck_elevation_ft: c.deck_ft.map(step5),
                feature: c.feature,
                road_class: c.class,
            });
        }
    }
    let mut walls: BTreeMap<(u32, u32, EdgeAxis), &Wall> = BTreeMap::new();
    for wall in &g.walls {
        let (lx, ly) = (wall.gx - win.gx0, wall.gy - win.gy0);
        let (mx, my) = match wall.axis {
            EdgeAxis::Horizontal => (i64::from(w) - 1, i64::from(h)),
            EdgeAxis::Vertical => (i64::from(w), i64::from(h) - 1),
        };
        if lx < 0 || ly < 0 || lx > mx || ly > my {
            continue;
        }
        if let (Ok(x), Ok(y)) = (u32::try_from(lx), u32::try_from(ly)) {
            walls.insert((x, y, wall.axis), wall);
        }
    }
    let mut edges = Vec::new();
    for ((x, y, axis), wall) in walls {
        layout.walls.push(WallSegment {
            x,
            y,
            axis,
            kind: wall.kind,
            kit: wall.kit.to_string(),
        });
        let solid = wall.role == EdgeRole::Building && wall.kind != WallRole::Door;
        edges.push(EdgeRule {
            x,
            y,
            axis,
            role: wall.role,
            blocks_movement: wall.kind != WallRole::Door,
            blocks_sight: solid && wall.kind != WallRole::Window,
        });
    }
    // Earlier placements (trees, rocks) cannot stand on the new ways.
    layout.placements.retain(|p| {
        #[allow(clippy::cast_possible_truncation)] // anchors are small
        let (x, y) = (p.x.floor() as i64, p.y.floor() as i64);
        g.get(win.gx0 + x, win.gy0 + y)
            .is_none_or(|c| !clears_props(c.feature))
    });
    #[allow(clippy::cast_precision_loss)] // window indices are small
    let (ox, oy) = (win.gx0 as f64, win.gy0 as f64);
    for p in &g.props {
        let (lx, ly) = (quant(p.x - ox), quant(p.y - oy));
        let inside = lx >= -PROP_MARGIN
            && ly >= -PROP_MARGIN
            && lx < f64::from(w) + PROP_MARGIN
            && ly < f64::from(h) + PROP_MARGIN;
        if inside {
            #[allow(clippy::cast_possible_truncation)] // quantised small values
            layout.placements.push(Placement {
                asset: AssetRef::Id(p.id.to_string()),
                x: lx as f32,
                y: ly as f32,
                rotation: p.rotation,
                mirror: false,
            });
        }
    }
    for &(x, y, r) in &g.lights {
        let (lx, ly) = (quant(x - ox), quant(y - oy));
        if lx >= 0.0 && ly >= 0.0 && lx < f64::from(w) && ly < f64::from(h) {
            #[allow(clippy::cast_possible_truncation)] // quantised small values
            layout.lights.push(LightSource {
                x: lx as f32,
                y: ly as f32,
                radius_ft: r,
                colour: [255, 190, 120],
            });
        }
    }
    Sidecar {
        format_version: SIDECAR_VERSION,
        layout: layout.name.clone(),
        width: w,
        height: h,
        squares,
        edges,
    }
}

/// Features on which earlier props are removed.
fn clears_props(f: Feature) -> bool {
    !matches!(f, Feature::None | Feature::Bank | Feature::Shoulder)
}

/// Rounds to 1/8 square, exact in `f32`.
fn quant(v: f64) -> f64 {
    (v * 8.0).round() / 8.0
}
