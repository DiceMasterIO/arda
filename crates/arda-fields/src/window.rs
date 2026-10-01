//! `fields_window`: a countryside window as a [`TacticalLayout`] plus its
//! SRD sidecar.

use crate::boundary::{self, Boundaries};
use crate::dress::{dress, Dressed};
use crate::error::FieldsError;
use crate::fields::FieldKind;
use crate::geom::{Sq, SQUARE_M};
use crate::input::FieldInputs;
use crate::plan::{Cover, Plan};
use crate::sidecar::{
    difficult_ground, edge_rules, placement_rules, CompoundRecord, EdgeRules, FieldRecord, Sidecar,
    SquareRules, SrdCover, SIDECAR_VERSION,
};
use arda_tactical::layout::{
    AssetRef, EdgeAxis, LightSource, Placement, Square, TacticalLayout, WallSegment,
};
use std::collections::BTreeSet;

/// Largest window side, in squares.
pub const MAX_SIDE: u32 = 2048;
const FT_PER_M: f64 = 3.280_84;
/// Elevations are absolute feet in 5 ft steps (`vocabulary.md`, I20).
const ELEV_STEP_FT: f64 = 5.0;

/// A generated window: the layout and its sidecar.
#[derive(Debug, Clone)]
pub struct FieldsWindow {
    /// The layout, in canonical vocabulary keys.
    pub layout: TacticalLayout,
    /// SRD rules and records.
    pub sidecar: Sidecar,
    /// The same rules as `arda-scene`'s `RulesSidecar` format 2 (A8, I9),
    /// stated on owned and fringe squares only.
    pub rules: arda_scene::RulesSidecar,
    /// Row-major: whether this layer reserves the square (fields, compounds,
    /// lanes and aprons; logic/09 §reservations precedence 4). Roads, water,
    /// built cores and wild land are left to their owners.
    pub owned: Vec<bool>,
    /// Row-major: depth into the woodland fringe (`0` beside worked
    /// ground, towards `1` at the forest), `None` outside it
    /// ([`crate::fringe`]). The layout already dresses these squares as
    /// rough grass and scrub; they are not in `owned`, since only a caller
    /// that knows the natural cover beneath can tell whether they apply.
    pub fringe: Vec<Option<f32>>,
    /// Per-square height of built earthworks over the natural ground, in
    /// feet (quarry pits, spoil heaps, adit faces), row-major. Renderers
    /// that shade natural slopes themselves can use this instead of the
    /// absolute elevation.
    pub earthworks_ft: Vec<i16>,
}

/// The countryside of a window as a tactical layout.
///
/// `window_origin_m` is snapped down to the square lattice; `w × h` squares.
///
/// # Errors
/// An empty, oversized or non-finite window.
pub fn fields_window(
    inputs: &FieldInputs<'_>,
    window_origin_m: [f64; 2],
    w: u32,
    h: u32,
    seed: u64,
) -> Result<TacticalLayout, FieldsError> {
    generate(inputs, window_origin_m, w, h, seed).map(|f| f.layout)
}

/// The window rectangle in global squares.
///
/// # Errors
/// As [`fields_window`].
#[allow(clippy::cast_possible_truncation)] // checked finite map positions
pub fn window_rect(
    origin_m: [f64; 2],
    w: u32,
    h: u32,
) -> Result<(i64, i64, i64, i64), FieldsError> {
    if w == 0 || h == 0 || w > MAX_SIDE || h > MAX_SIDE {
        return Err(FieldsError::WindowSize {
            width: w,
            height: h,
            max: MAX_SIDE,
        });
    }
    let (ox, oy) = (origin_m[0] / SQUARE_M, origin_m[1] / SQUARE_M);
    if !ox.is_finite() || !oy.is_finite() || ox.abs() > 1e12 || oy.abs() > 1e12 {
        return Err(FieldsError::Origin(origin_m[0], origin_m[1]));
    }
    // Snap within a hair of a lattice line to that line, so metre origins
    // computed as `k × SQUARE_M` land on square k.
    let snap = |v: f64| (v + 1e-9).floor() as i64;
    Ok((snap(ox), snap(oy), i64::from(w), i64::from(h)))
}

/// The layout, sidecar and intermediate plan of a window.
///
/// # Errors
/// As [`fields_window`].
pub fn generate(
    inputs: &FieldInputs<'_>,
    window_origin_m: [f64; 2],
    w: u32,
    h: u32,
    seed: u64,
) -> Result<FieldsWindow, FieldsError> {
    let win = window_rect(window_origin_m, w, h)?;
    let plan = Plan::build(inputs, win, seed);
    let bounds = boundary::build(&plan);
    let dressed = dress(&plan, &bounds);
    let name = format!("fields_{}_{}_{w}x{h}", win.0, win.1);
    let mut layout = layout(&plan, &bounds, &dressed, &name, w, h);
    // Convention I10: the layout carries its world origin in squares.
    layout.origin = Some([win.0, win.1]);
    let fringe = crate::fringe::fringe(&plan);
    dress_fringe(&plan, &fringe, &mut layout);
    let sidecar = sidecar(&plan, &bounds, &dressed, &layout);
    let owned: Vec<bool> = dressed
        .ground
        .squares()
        .map(|s| match plan.at(s) {
            // Unused land keeps the refined natural ground: its ecology,
            // rocks and trees follow the terrain, not a field's paint.
            Cover::Field(_) => plan
                .field_at(s)
                .is_some_and(|(_, f)| f.kind != FieldKind::Wild),
            Cover::Compound(_) | Cover::Lane | Cover::Apron => true,
            _ => false,
        })
        .collect();
    // Rules are stated on the fringe too, for callers that take it.
    let stated: Vec<bool> = owned
        .iter()
        .zip(&fringe)
        .map(|(&o, f)| o || f.is_some())
        .collect();
    let rules = sidecar.to_rules(&stated);
    let earthworks_ft = dressed
        .ground
        .squares()
        .zip(&layout.squares)
        .map(|(s, sq)| {
            let t = plan.terrain.get(s).copied().unwrap_or_default();
            sq.elevation_ft - feet(t.height_m, 0, ELEV_STEP_FT)
        })
        .collect();
    Ok(FieldsWindow {
        layout,
        sidecar,
        rules,
        owned,
        fringe,
        earthworks_ft,
    })
}

/// Grounds and young trees of the fringe: grass beside the fields, scrub
/// towards the forest, with birch and bushes scattered through it.
fn dress_fringe(plan: &Plan, fringe: &[Option<f32>], layout: &mut TacticalLayout) {
    use crate::geom::{h2, u01};
    let (x0, y0, w, _) = plan.win;
    for (i, t) in fringe.iter().enumerate() {
        let Some(t) = *t else { continue };
        let k = i64::try_from(i).unwrap_or(0);
        let (x, y) = (x0 + k % w, y0 + k / w);
        let n = crate::dress::patch(plan.seed, 0xF41F, Sq::new(x, y), 0.15);
        let v = f64::from(t) + 0.6 * (n - 0.5);
        let ground = if v < 0.4 {
            "grass"
        } else if v < 0.62 {
            "meadow"
        } else {
            "scrub"
        };
        if let Some(sq) = layout.squares.get_mut(i) {
            ground.clone_into(&mut sq.ground);
        }
        let u = u01(h2(plan.seed, 0xF420, x, y));
        let asset = if u < 0.012 + 0.03 * f64::from(t) {
            "veg.tree_birch"
        } else if u < 0.05 + 0.08 * f64::from(t) {
            "veg.bush"
        } else if u < 0.09 + 0.05 * f64::from(t) {
            "veg.tall_grass"
        } else {
            continue;
        };
        #[allow(clippy::cast_precision_loss)] // window-local squares
        let (lx, ly) = ((x - x0) as f32 + 0.5, (y - y0) as f32 + 0.5);
        layout.placements.push(Placement {
            asset: AssetRef::Id(asset.to_string()),
            x: lx,
            y: ly,
            rotation: 0,
            mirror: false,
        });
    }
}

/// Metres to feet, rounded to `step` feet and clamped.
#[allow(clippy::cast_possible_truncation)] // clamped feet
fn feet(m: f64, delta_ft: i16, step: f64) -> i16 {
    ((m * FT_PER_M + f64::from(delta_ft)) / step)
        .round()
        .mul_add(step, 0.0)
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn local(plan: &Plan, s: Sq) -> Option<(u32, u32)> {
    let (x0, y0, _, _) = plan.win;
    Some((u32::try_from(s.x - x0).ok()?, u32::try_from(s.y - y0).ok()?))
}

/// Whether a global edge key lies on the window (far borders included).
fn edge_in(plan: &Plan, axis: EdgeAxis, x: i64, y: i64) -> bool {
    let (x0, y0, w, h) = plan.win;
    match axis {
        EdgeAxis::Vertical => x >= x0 && x <= x0 + w && y >= y0 && y < y0 + h,
        EdgeAxis::Horizontal => x >= x0 && x < x0 + w && y >= y0 && y <= y0 + h,
    }
}

fn layout(
    plan: &Plan,
    bounds: &Boundaries,
    dressed: &Dressed,
    name: &str,
    w: u32,
    h: u32,
) -> TacticalLayout {
    let mut l = TacticalLayout::new(name, w, h, "grass");
    let (x0, y0, _, _) = plan.win;
    #[allow(clippy::cast_precision_loss)] // window offsets
    let (fx, fy) = (x0 as f64, y0 as f64);
    for s in dressed.ground.squares() {
        let Some((lx, ly)) = local(plan, s) else {
            continue;
        };
        let t = plan.terrain.get(s).copied().unwrap_or_default();
        let mut sq = Square {
            ground: (*dressed.ground.get(s).unwrap_or(&"grass")).to_string(),
            elevation_ft: feet(t.height_m, 0, ELEV_STEP_FT),
            water_depth_ft: 0,
        };
        if plan.at(s) == Cover::Water {
            let d = feet(t.water_depth_m, 0, 1.0).clamp(1, 255);
            sq.water_depth_ft = u8::try_from(d).unwrap_or(1);
        }
        if let Cover::Compound(c) = plan.at(s) {
            if let Some(cs) = plan
                .compounds
                .get(c as usize)
                .and_then(|c| c.squares.get(&s))
            {
                sq.elevation_ft = feet(t.height_m, cs.elev_ft, ELEV_STEP_FT);
                if cs.water_ft > 0 {
                    sq.water_depth_ft = sq.water_depth_ft.max(cs.water_ft);
                }
            }
        }
        if let Some(slot) = l.square_mut(lx, ly) {
            *slot = sq;
        }
    }
    for (e, info) in &bounds.walls {
        if !edge_in(plan, e.axis, e.x, e.y) {
            continue;
        }
        l.walls.push(WallSegment {
            x: u32::try_from(e.x - x0).unwrap_or(0),
            y: u32::try_from(e.y - y0).unwrap_or(0),
            axis: e.axis,
            kind: info.role,
            kit: info.kit.to_string(),
        });
    }
    #[allow(clippy::cast_possible_truncation)] // window-local positions
    for p in &dressed.placements {
        l.placements.push(Placement {
            asset: p.asset.clone(),
            x: (p.x - fx) as f32,
            y: (p.y - fy) as f32,
            rotation: p.rotation,
            mirror: p.mirror,
        });
    }
    #[allow(clippy::cast_possible_truncation)] // window-local positions
    for c in &plan.compounds {
        for (p, radius_ft, colour) in &c.lights {
            if plan.in_window(Sq::containing(*p)) {
                l.lights.push(LightSource {
                    x: (p[0] - fx) as f32,
                    y: (p[1] - fy) as f32,
                    radius_ft: *radius_ft,
                    colour: *colour,
                });
            }
        }
    }
    l
}

/// The row-major index of the square holding window-local point `(x, y)`,
/// or `None` off the window. An f32 position rounded up to `width` would
/// otherwise land on the first square of the next row.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // range-checked
fn square_of(x: f32, y: f32, width: u32, height: u32) -> Option<usize> {
    let (fx, fy) = (x.floor(), y.floor());
    let inside = |v: f32, n: u32| v >= 0.0 && v < n as f32;
    (inside(fx, width) && inside(fy, height)).then(|| fy as usize * width as usize + fx as usize)
}

fn sidecar(plan: &Plan, bounds: &Boundaries, dressed: &Dressed, l: &TacticalLayout) -> Sidecar {
    let (x0, y0, w, h) = plan.win;
    let mut stands = vec![(SrdCover::None, false, false, false); l.squares.len()];
    for p in &l.placements {
        let AssetRef::Id(id) = &p.asset else {
            continue;
        };
        let Some(slot) = square_of(p.x, p.y, l.width, l.height).and_then(|i| stands.get_mut(i))
        else {
            continue;
        };
        let r = placement_rules(id);
        *slot = (slot.0.max(r.0), slot.1 || r.1, slot.2 || r.2, slot.3 || r.3);
    }
    let mut squares = Vec::with_capacity(l.squares.len());
    let mut seen = BTreeSet::new();
    let mut touching = BTreeSet::new();
    for ((s, sq), st) in dressed.ground.squares().zip(&l.squares).zip(&stands) {
        let field = plan.field_at(s);
        if let Some((i, _)) = field {
            touching.insert(i);
        }
        if let Cover::Compound(c) = plan.at(s) {
            seen.insert(c as usize);
        }
        squares.push(SquareRules {
            ground: sq.ground.clone(),
            difficult: difficult_ground(&sq.ground) || (1..5).contains(&sq.water_depth_ft) || st.3,
            water_depth_ft: sq.water_depth_ft,
            cover: st.0,
            blocks_sight: st.1,
            // Deep water is swum (I15), never impassable.
            blocks_movement: st.2,
            deck: false,
            crop: dressed.crop.get(s).copied().flatten(),
            furrow: dressed.furrow.get(s).copied().flatten(),
            field: field.map(|(_, f)| f.id.to_string()),
        });
    }
    let edges = l
        .walls
        .iter()
        .map(|wseg| {
            let (blocks_sight, blocks_movement, cover, climb, openable) =
                edge_rules(&wseg.kit, wseg.kind);
            EdgeRules {
                x: wseg.x,
                y: wseg.y,
                axis: wseg.axis,
                kind: wseg.kind,
                kit: wseg.kit.clone(),
                blocks_sight,
                blocks_movement,
                cover,
                climb,
                openable,
            }
        })
        .collect();
    let fields = touching
        .into_iter()
        .filter_map(|i| plan.fields.get(i).map(|f| (i, f)))
        .map(|(i, f)| FieldRecord {
            id: f.id.to_string(),
            kind: f.kind,
            crop: f.crop,
            kit: f.kit.to_string(),
            hectares: f.hectares(),
            enclosed: f.kind.enclosed(),
            gates: bounds.gates.get(i).map_or(0, Vec::len),
            complete: field_complete(plan, bounds, i),
        })
        .collect();
    let compounds = seen
        .into_iter()
        .filter_map(|i| plan.compounds.get(i).map(|c| (i, c)))
        .map(|(i, c)| CompoundRecord {
            kind: c.kind,
            cell: [c.cell.0, c.cell.1],
            lane: plan.lanes.get(i).is_some_and(Option::is_some),
        })
        .collect();
    Sidecar {
        format_version: SIDECAR_VERSION,
        name: l.name.clone(),
        seed: plan.seed.to_string(),
        width: u32::try_from(w).unwrap_or(0),
        height: u32::try_from(h).unwrap_or(0),
        origin_square: [x0, y0],
        square_m: SQUARE_M,
        squares,
        edges,
        fields,
        compounds,
    }
}

/// Whether every square of field `i` lies inside the window.
fn field_complete(plan: &Plan, bounds: &Boundaries, i: usize) -> bool {
    let Some(f) = plan.fields.get(i) else {
        return false;
    };
    if !plan.in_window(f.first) {
        return false;
    }
    match bounds.edges.get(i) {
        Some(edges) if f.kind.enclosed() => edges.iter().all(|(e, _)| {
            let (a, b) = e.sides();
            plan.in_window(a) && plan.in_window(b)
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placement_on_the_east_edge_stays_on_its_row() {
        assert_eq!(square_of(0.5, 0.5, 4, 3), Some(0));
        assert_eq!(square_of(3.99, 1.2, 4, 3), Some(7));
        assert_eq!(square_of(4.0, 1.2, 4, 3), None, "not square (0, 2)");
        assert_eq!(square_of(1.0, 3.0, 4, 3), None);
        assert_eq!(square_of(-0.1, 1.0, 4, 3), None, "not square (0, 1)");
        assert_eq!(square_of(f32::NAN, 1.0, 4, 3), None);
    }
}
