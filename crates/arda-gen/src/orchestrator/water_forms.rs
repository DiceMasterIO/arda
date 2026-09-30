//! Stored river and lake forms for each area (logic/02 §world-water
//! publication, goals 8-13).
//!
//! Each river segment gets its hydraulic geometry from its saved mean
//! discharge (width `4 m × Q^0.5`, depth `0.3 m × Q^0.4`), its sinuosity
//! and slope measured on the saved 100 m course, and a planform: braided
//! where formation built a braidplain under at least half of its course,
//! meandering where the measured sinuosity reaches 1.25 below the
//! braiding slope (steeper sinuous courses follow winding valleys), and
//! straight otherwise. Each lake fragment gets the origin of the formation sink it
//! holds and whether its connected lake is terminal (no annual outflow, so
//! evaporation-balanced and saline). Deltas and dolines are listed by the
//! area holding them.

use std::collections::BTreeMap;

use arda_core::hydrology::{channel_width_dm, GlobalLake};
use arda_core::water::{
    bankfull_depth_cm, AreaWater, ChannelPattern, DeltaForm, Doline, LakeForm, LakeOrigin,
    SegmentForm,
};
use arda_core::{AreaCells, AreaCoord, AreaObjects, BasinId, CellCoord, Litres, AREA_CELLS};

use crate::formation::water::{SinkKind, WaterFeatures};

/// Measured sinuosity at which a single-thread course counts as meandering, ‰.
pub const MEANDERING_PERMILLE: u16 = 1_250;
const CELL_UM: i64 = 100_000_000;

fn global_cell(x_um: i64, y_um: i64) -> (i64, i64) {
    (
        (x_um + CELL_UM / 2).div_euclid(CELL_UM),
        (y_um + CELL_UM / 2).div_euclid(CELL_UM),
    )
}

fn origin_of(kind: SinkKind) -> LakeOrigin {
    match kind {
        SinkKind::Tectonic => LakeOrigin::Tectonic,
        SinkKind::Glacial => LakeOrigin::Glacial,
        SinkKind::Oxbow => LakeOrigin::Oxbow,
        SinkKind::Karst => LakeOrigin::Karst,
    }
}

/// Forms of one area. Lake origins found here are also recorded by basin
/// in `origins`, so fragments of a lake whose sink lies in another area can
/// be resolved by [`resolve_origins`].
#[must_use]
pub fn area_water(
    area: AreaCoord,
    cells: &AreaCells,
    objects: &AreaObjects,
    features: &WaterFeatures,
    lakes: &[GlobalLake],
    origins: &mut BTreeMap<BasinId, LakeOrigin>,
) -> AreaWater {
    let (ox, oy) = (i64::from(area.x) * 512, i64::from(area.y) * 512);
    let global = |c: CellCoord| (i64::from(c.x()) + ox, i64::from(c.y()) + oy);
    let local = |x: i64, y: i64| {
        let (lx, ly) = (x - ox, y - oy);
        if (0..i64::from(AREA_CELLS)).contains(&lx) && (0..i64::from(AREA_CELLS)).contains(&ly) {
            CellCoord::new(u16::try_from(lx).ok()?, u16::try_from(ly).ok()?)
        } else {
            None
        }
    };
    let braided = |c: CellCoord| -> Option<u32> {
        let (x, y) = global(c);
        let key = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
        let i = features
            .braided_cells
            .binary_search_by_key(&key, |&(bx, by, _)| (bx, by))
            .ok()?;
        Some(features.braided_cells[i].2)
    };
    let mut out = AreaWater::default();
    let windows = arda_core::water::course_windows(&objects.rivers);
    for (r, win) in objects.rivers.iter().zip(&windows) {
        let sin = win.sinuosity_permille;
        let drop = i64::from(cells.get(win.first).height.raw())
            - i64::from(cells.get(win.last).height.raw());
        // mm over run × 100 m (run in thousandths of a cell): ppm.
        let slope = drop.max(0) * 10_000 / win.run_permille.max(1);
        let belts: Vec<u32> = r.course.iter().filter_map(|&c| braided(c)).collect();
        let width = channel_width_dm(r.discharge).unwrap_or(u32::MAX);
        let pattern = if belts.len() * 2 >= r.course.len() && !belts.is_empty() {
            ChannelPattern::Braided
        } else if sin >= MEANDERING_PERMILLE
            && slope < i64::from(arda_core::water::braiding_slope_ppm(r.discharge))
        {
            ChannelPattern::Meandering
        } else {
            ChannelPattern::Straight
        };
        out.segments.push(SegmentForm {
            pattern,
            bankfull_width_dm: width,
            bankfull_depth_cm: bankfull_depth_cm(r.discharge).unwrap_or(u32::MAX),
            belt_width_dm: if pattern == ChannelPattern::Braided {
                belts
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0)
                    .saturating_mul(10)
                    .max(width)
            } else {
                width
            },
            sinuosity_permille: sin,
            slope_ppm: u32::try_from(slope).unwrap_or(u32::MAX),
        });
    }
    for l in &objects.lakes {
        let mut origin = LakeOrigin::Unclassified;
        for s in &features.sinks {
            let reach = s.radius_um + CELL_UM;
            let hit = l.cells.iter().any(|&c| {
                let (x, y) = global(c);
                let (dx, dy) = (
                    i128::from(x * CELL_UM - s.x_um),
                    i128::from(y * CELL_UM - s.y_um),
                );
                dx * dx + dy * dy <= i128::from(reach) * i128::from(reach)
            });
            if hit {
                let o = origin_of(s.kind);
                if origin == LakeOrigin::Unclassified || o < origin {
                    origin = o;
                }
            }
        }
        if origin != LakeOrigin::Unclassified {
            let e = origins.entry(l.global_id).or_insert(origin);
            *e = (*e).min(origin);
        }
        let terminal = lakes
            .iter()
            .find(|g| g.basin == l.global_id)
            .is_some_and(|g| g.annual_outflow == Litres(0));
        out.lakes.push(LakeForm { origin, terminal });
    }
    for d in &features.deltas {
        let (x, y) = global_cell(d.apex_um.0, d.apex_um.1);
        if let Some(apex) = local(x, y) {
            out.deltas.push(DeltaForm {
                apex,
                radius_m: u32::try_from(d.radius_m).unwrap_or(u32::MAX),
                catchment_km2: u32::try_from(d.catchment_km2).unwrap_or(u32::MAX),
            });
        }
    }
    for d in &features.dolines {
        let (x, y) = global_cell(d.x_um, d.y_um);
        if let Some(at) = local(x, y) {
            if cells.get(at).terrain == arda_core::TerrainKind::Land {
                out.dolines.push(Doline {
                    at,
                    radius_dm: u16::try_from(d.radius_mm / 100).unwrap_or(u16::MAX),
                    depth_dm: u16::try_from(d.depth_mm / 100).unwrap_or(u16::MAX),
                });
            }
        }
    }
    out
}

/// Gives every unclassified fragment the origin found for its basin in any
/// area.
pub fn resolve_origins(
    water: &mut AreaWater,
    objects_lakes: &[BasinId],
    origins: &BTreeMap<BasinId, LakeOrigin>,
) {
    for (form, id) in water.lakes.iter_mut().zip(objects_lakes) {
        if form.origin == LakeOrigin::Unclassified {
            if let Some(&o) = origins.get(id) {
                form.origin = o;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formation::water::Sink;
    use arda_core::hydrology::ReachId;
    use arda_core::{Cell, DischargeMilli, HeightMm, Lake, RiverSegment, Terminus, TerrainKind};

    fn c(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    /// One area with a 30-cell river running east at 2 m per cell, a lake
    /// holding a formation sink, and a braided belt under the river's
    /// first cells.
    fn fixture(annual_outflow: u128) -> (AreaCells, AreaObjects, WaterFeatures, Vec<GlobalLake>) {
        let mut cells = AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            ..Cell::default()
        });
        let mut objects = AreaObjects::empty();
        for i in 0..30_u16 {
            let at = c(100 + i, 50);
            cells.set(
                at,
                Cell {
                    height: HeightMm::new(100_000 - i32::from(i) * 2_000),
                    terrain: TerrainKind::Land,
                    ..Cell::default()
                },
            );
            objects.rivers.push(RiverSegment {
                global_id: ReachId(u64::from(i) + 1),
                id: u32::from(i) + 1,
                order: 2,
                width_dm: 40,
                discharge: DischargeMilli::new(1_000 + u64::from(i) * 100),
                feeds: (i < 29).then_some(u32::from(i) + 2),
                ends: if i < 29 {
                    Terminus::Junction
                } else {
                    Terminus::Sea
                },
                course: vec![at],
            });
        }
        objects.lakes.push(Lake {
            global_id: BasinId(7),
            id: 1,
            surface: HeightMm::new(5_000),
            depth_mm: 2_000,
            outlet: None,
            cells: vec![c(300, 300), c(301, 300), c(300, 301), c(301, 301)],
        });
        let mut features = WaterFeatures::default();
        features.sinks.push(Sink {
            x_um: 300 * CELL_UM,
            y_um: 301 * CELL_UM,
            radius_um: 75_000_000,
            kind: SinkKind::Oxbow,
        });
        for i in 0..12 {
            features.braided_cells.push((100 + i, 50, 400));
        }
        features.index();
        let lakes = vec![GlobalLake {
            basin: BasinId(7),
            surface: HeightMm::new(5_000),
            deepest_bed: HeightMm::new(3_000),
            submerged_cells: 4,
            outlet: None,
            annual_outflow: Litres(annual_outflow),
            mean_outflow: DischargeMilli::new(0),
        }];
        (cells, objects, features, lakes)
    }

    #[test]
    fn segments_carry_hydraulic_geometry_slope_and_planform() {
        let (cells, objects, features, lakes) = fixture(0);
        let mut origins = BTreeMap::new();
        let w = area_water(
            AreaCoord::new(0, 0),
            &cells,
            &objects,
            &features,
            &lakes,
            &mut origins,
        );
        assert_eq!(w.segments.len(), 30);
        let first = w.segments[0];
        assert_eq!(first.bankfull_width_dm, 40, "4 m at 1 m³/s");
        assert_eq!(first.bankfull_depth_cm, 30, "0.3 m at 1 m³/s");
        // 2 m per 100 m cell.
        assert!(
            (19_000..=21_000).contains(&first.slope_ppm),
            "{}",
            first.slope_ppm
        );
        assert_eq!(first.sinuosity_permille, 1_000);
        assert_eq!(first.pattern, ChannelPattern::Braided);
        assert_eq!(first.belt_width_dm, 4_000);
        assert_eq!(w.segments[29].pattern, ChannelPattern::Straight);
        // Depth and width grow downstream with discharge.
        assert!(w.segments[29].bankfull_depth_cm > first.bankfull_depth_cm);
        assert!(w.segments[29].bankfull_width_dm > first.bankfull_width_dm);
    }

    #[test]
    fn lakes_take_their_sink_origin_and_terminal_state() {
        let (cells, objects, features, lakes) = fixture(0);
        let mut origins = BTreeMap::new();
        let w = area_water(
            AreaCoord::new(0, 0),
            &cells,
            &objects,
            &features,
            &lakes,
            &mut origins,
        );
        assert_eq!(w.lakes[0].origin, LakeOrigin::Oxbow);
        assert!(
            w.lakes[0].terminal,
            "no annual outflow: a terminal, saline lake"
        );
        assert_eq!(origins.get(&BasinId(7)), Some(&LakeOrigin::Oxbow));
        let (cells, objects, features, lakes) = fixture(1_000_000);
        let w = area_water(
            AreaCoord::new(0, 0),
            &cells,
            &objects,
            &features,
            &lakes,
            &mut origins,
        );
        assert!(!w.lakes[0].terminal, "an overflowing lake is open");
        // A fragment elsewhere takes the origin found for its basin.
        let mut far = AreaWater {
            lakes: vec![LakeForm::default()],
            ..AreaWater::default()
        };
        resolve_origins(&mut far, &[BasinId(7)], &origins);
        assert_eq!(far.lakes[0].origin, LakeOrigin::Oxbow);
    }
}
