//! The three society layers of one window.

use super::town::{occupies, plans_near, Streets};
use super::{Bbox, SQUARE_M, WAYS_MARGIN_M};
use crate::overlays::{OverlayCtx, OverlayLayer};
use crate::BlocksError;
use arda_fields::{LandUse as FieldUse, LandUseMap, TerrainSample};
use arda_ids::LandUse;
use arda_people::terrain::Surroundings;
use arda_people::World;
use arda_ways::Feature;
use std::path::Path;

fn err(layer: &'static str) -> impl Fn(String) -> BlocksError {
    move |message| BlocksError::Overlay { layer, message }
}

type Converted = (
    Vec<(Bbox, arda_ways::Road)>,
    Vec<arda_ways::Crossing>,
    Vec<(Bbox, arda_fields::Road)>,
);

/// Settle's roads and crossings as `arda-ways` and `arda-fields` records.
/// A road's surface wealth is that of the settlement it starts from.
pub fn roads(world: &World) -> Result<Converted, BlocksError> {
    let e = err("ways");
    let wealth = |id: u64| world.files.settlement(id).map_or(128, |(s, _)| s.wealth);
    let mut ways = Vec::new();
    let mut fields = Vec::new();
    for r in &world.files.roads.roads {
        let json = serde_json::to_value(r).map_err(|x| e(x.to_string()))?;
        let mut road: arda_ways::Road =
            serde_json::from_value(json).map_err(|x| e(format!("road {}: {x}", r.id)))?;
        road.wealth = wealth(r.from);
        #[allow(clippy::cast_precision_loss)] // world metres are far below 2^52
        let pts = |seg: &Vec<[i64; 2]>| -> Vec<[f64; 2]> {
            seg.iter().map(|p| [p[0] as f64, p[1] as f64]).collect()
        };
        let bbox = Bbox::of(r.segments.iter().flat_map(&pts));
        ways.push((bbox, road));
        let class = arda_fields::RoadClass::from_code(r.class.code());
        for seg in &r.segments {
            let points = pts(seg);
            fields.push((
                Bbox::of(points.iter().copied()),
                arda_fields::Road { class, points },
            ));
        }
    }
    let crossings = world
        .files
        .roads
        .crossings
        .iter()
        .map(|c| {
            let json = serde_json::to_value(c).map_err(|x| e(x.to_string()))?;
            serde_json::from_value(json).map_err(|x| e(format!("crossing {}: {x}", c.id)))
        })
        .collect::<Result<_, _>>()?;
    Ok((ways, crossings, fields))
}

/// `landuse.bin` codes as `arda-fields` classes (adapter A17): built
/// ground is left to the town layer, wild ground has no use.
#[must_use]
pub const fn fields_class(code: u8) -> Option<FieldUse> {
    match LandUse::from_code(code) {
        Some(LandUse::Field) => Some(FieldUse::Arable),
        Some(LandUse::Pasture) => Some(FieldUse::Pasture),
        Some(LandUse::Orchard) => Some(FieldUse::Orchard),
        Some(LandUse::Woodland) => Some(FieldUse::Woodland),
        Some(LandUse::Mill) => Some(FieldUse::Mill),
        Some(LandUse::Mine) => Some(FieldUse::MineQuarry),
        Some(LandUse::Meadow) => Some(FieldUse::Meadow),
        Some(LandUse::Fallow) => Some(FieldUse::Fallow),
        Some(LandUse::Farmstead) => Some(FieldUse::Farmstead),
        Some(LandUse::None | LandUse::Built) | None => None,
    }
}

/// The world's land-use raster, one code per 100 m cell.
#[derive(Debug, Clone)]
pub struct LandUseRaster {
    width: usize,
    height: usize,
    codes: Vec<u8>,
    /// Built cells of hamlets and villages: their tofts and crofts are
    /// farmed wherever the town plan leaves the ground free.
    crofts: Vec<bool>,
}

impl LandUseRaster {
    /// Reads `landuse.bin`; `rural(owner)` says whether a built cell's
    /// owning settlement is a hamlet or village.
    ///
    /// # Errors
    /// [`BlocksError::Overlay`] for an unreadable or malformed raster.
    pub fn read(path: &Path, rural: impl Fn(u64) -> bool) -> Result<Self, BlocksError> {
        let (width, height, codes, owners) =
            arda_settle::output::read_landuse(path).map_err(|e| err("fields")(e.to_string()))?;
        let built = LandUse::Built.code();
        let crofts = codes
            .iter()
            .zip(&owners)
            .map(|(&c, &o)| c == built && rural(u64::from(o)))
            .collect();
        Ok(Self {
            width,
            height,
            codes,
            crofts,
        })
    }
}

impl LandUseMap for LandUseRaster {
    fn class_at(&self, cx: i64, cy: i64) -> Option<FieldUse> {
        let x = usize::try_from(cx).ok().filter(|&x| x < self.width)?;
        let y = usize::try_from(cy).ok().filter(|&y| y < self.height)?;
        let i = y * self.width + x;
        if self.crofts.get(i).copied().unwrap_or(false) {
            return Some(FieldUse::Arable);
        }
        fields_class(*self.codes.get(i)?)
    }
}

/// The world cells around the window, far enough out for every query a
/// layer makes.
fn surroundings(ctx: &OverlayCtx<'_>, world: &World) -> Result<Surroundings, BlocksError> {
    let b = Bbox::of_window(ctx);
    let centre = ((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
    let radius = (b.x1 - b.x0).hypot(b.y1 - b.y0) / 2.0 + WAYS_MARGIN_M + 300.0;
    Surroundings::around(&world.src, centre, radius).map_err(|e| err("ways")(e.to_string()))
}

#[allow(clippy::cast_precision_loss)] // squares are far below 2^52
fn origin_m(ctx: &OverlayCtx<'_>) -> [f64; 2] {
    [ctx.gsx0 as f64 * SQUARE_M, ctx.gsy0 as f64 * SQUARE_M]
}

/// Roads, bridges and fords laid by `arda-ways` on the refined window, with
/// crossings found on the refined rivers themselves (`water`).
pub fn ways(
    ctx: &OverlayCtx<'_>,
    world: &World,
    roads: &[arda_ways::Road],
    crossings: &[arda_ways::Crossing],
    water: &arda_refine::water::RiverWater,
) -> Result<OverlayLayer, BlocksError> {
    let s = surroundings(ctx, world)?;
    let terrain = super::rivers::WaysTerrain::new(&s, water, &world.src);
    let mut layout = ctx.base.clone();
    let out = arda_ways::apply_ways(
        &mut layout,
        origin_m(ctx),
        roads,
        crossings,
        &terrain,
        ctx.seed,
    )
    .map_err(|e| err("ways")(e.to_string()))?;
    let _ = arda_ways::fallback::degrade(&mut layout, ctx.library);
    layout.origin = ctx.base.origin;
    let streets = Streets::of(&plans_near(ctx, world, WAYS_MARGIN_M)?);
    let w = ctx.width as usize;
    let mut owned = Vec::with_capacity(out.sidecar.squares.len());
    let mut soft = Vec::with_capacity(owned.capacity());
    for (i, q) in out.sidecar.squares.iter().enumerate() {
        #[allow(clippy::cast_possible_wrap)] // window sides are small
        let (gx, gy) = (ctx.gsx0 + (i % w) as i64, ctx.gsy0 + (i / w) as i64);
        let (own, verge) = claim(q.feature);
        // Where a road enters a town its street replaces it; channels and
        // banks only guide crossings (the refined river is the real one).
        let own = own && !streets.replace(gx, gy);
        if own && verge {
            natural_verge(&ctx.base.squares[i].ground, &mut layout.squares[i].ground);
        }
        if own && !on_water(q.feature) {
            drain(&ctx.base.squares[i], &mut layout.squares[i]);
        }
        owned.push(own);
        soft.push(own && verge);
    }
    Ok(OverlayLayer {
        layout,
        rules: out.rules,
        owned,
        soft,
        elevation: false,
        review: Vec::new(),
    })
}

/// Whether the ways layer reserves a square of this feature, and whether
/// only softly (a verge or shoulder that fields may still take).
const fn claim(f: Feature) -> (bool, bool) {
    match f {
        Feature::None | Feature::Bank => (false, false),
        Feature::Verge | Feature::Shoulder => (true, true),
        _ => (true, false),
    }
}

/// Features that stand in water (the rest of a way is built up over it).
const fn on_water(f: Feature) -> bool {
    matches!(
        f,
        Feature::Bridge
            | Feature::Ford
            | Feature::Landing
            | Feature::FerryRope
            | Feature::GravelBar
    )
}

/// A way built over standing water the refined base holds (a pool or
/// marsh; rivers are crossed only by bridges and fords): dry, and never
/// the base's water ground.
fn drain(base: &arda_tactical::layout::Square, sq: &mut arda_tactical::layout::Square) {
    if base.water_depth_ft == 0 {
        return;
    }
    sq.water_depth_ft = 0;
    if sq.ground == base.ground {
        "mud".clone_into(&mut sq.ground);
    }
}

/// A verge that kept the natural ground: cleared forest floor becomes
/// grass.
fn natural_verge(base: &str, ground: &mut String) {
    if ground == base && matches!(base, "forest_floor" | "leaf_litter" | "moss") {
        "grass".clone_into(ground);
    }
}

/// Natural ground under trees.
fn wooded(ground: &str) -> bool {
    matches!(ground, "forest_floor" | "leaf_litter" | "moss")
}

fn fields_tier(t: arda_settle::model::Tier) -> arda_fields::Tier {
    use arda_settle::model::Tier as T;
    match t {
        T::Hamlet => arda_fields::Tier::Hamlet,
        T::Village => arda_fields::Tier::Village,
        T::Town => arda_fields::Tier::Town,
        T::City => arda_fields::Tier::City,
    }
}

/// Fields, pasture, orchards, mills and farmsteads from the land-use
/// raster. Culture, region and wealth are fixed so the layer depends only
/// on global inputs (goal 46).
pub fn fields(
    ctx: &OverlayCtx<'_>,
    world: &World,
    landuse: &LandUseRaster,
    roads: &[arda_fields::Road],
) -> Result<OverlayLayer, BlocksError> {
    let s = surroundings(ctx, world)?;
    let win = Bbox::of_window(ctx);
    #[allow(clippy::cast_precision_loss)]
    let settlements: Vec<arda_fields::Settlement> = world
        .files
        .settlements
        .settlements
        .iter()
        .filter(|x| Bbox::of([[x.x_m as f64, x.y_m as f64]]).near(&win, super::FIELDS_MARGIN_M))
        .map(|x| arda_fields::Settlement {
            x_m: x.x_m as f64,
            y_m: x.y_m as f64,
            tier: fields_tier(x.tier),
            population: x.population,
        })
        .collect();
    let terrain = |x: f64, y: f64| TerrainSample {
        height_m: s.height_m(x, y),
        water_depth_m: if s.open_water_at(x, y) {
            2.0
        } else if s.river_at(x, y).is_some() {
            1.0
        } else {
            0.0
        },
    };
    // Fields run up to the towns' own footprints (plans reach the fields'
    // whole planning margin).
    #[allow(clippy::cast_precision_loss)] // a small constant
    let margin = arda_fields::plan::MARGIN as f64 * SQUARE_M;
    let plans = plans_near(ctx, world, margin)?;
    let core = |gx: i64, gy: i64| {
        plans
            .iter()
            .filter_map(|(_, p)| p.as_ref().as_ref())
            .any(|p| occupies(p, gx, gy))
    };
    // Settle's rivers run straight between cell centres, the refined ones
    // meander: fields stop only at open water and meet the real banks.
    let barrier = |x: f64, y: f64| s.open_water_at(x, y);
    let inputs = arda_fields::FieldInputs {
        landuse,
        terrain: &terrain,
        culture: "human",
        region: arda_fields::Region::Mixed,
        wealth: 128,
        settlements: &settlements,
        roads,
        cores: Some(&core),
        barrier_water: Some(&barrier),
    };
    let win = arda_fields::generate(&inputs, origin_m(ctx), ctx.width, ctx.height, ctx.seed)
        .map_err(|e| err("fields")(e.to_string()))?;
    let (mut layout, _) = arda_fields::degrade::adapt(&win.layout, ctx.library);
    layout.origin = ctx.base.origin;
    // The fringe thins the forest back from worked ground; over open
    // natural ground there is nothing to thin.
    let owned = win
        .owned
        .iter()
        .zip(&win.fringe)
        .zip(&ctx.base.squares)
        .map(|((&o, f), b)| o || (f.is_some() && b.water_depth_ft == 0 && wooded(&b.ground)))
        .collect();
    Ok(OverlayLayer {
        layout,
        rules: win.rules,
        owned,
        soft: Vec::new(),
        elevation: false,
        review: Vec::new(),
    })
}
