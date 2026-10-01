//! The three demo layers, each a translation of one crate's sample into
//! the requested window (see the parent module).

use crate::overlays::{OverlayCtx, OverlayLayer};
use crate::BlocksError;
use arda_tactical::layout::TacticalLayout;
use arda_town::TownPlan;

/// Metres per square (convention I2: 100/64 m).
const SQUARE_M: f64 = 100.0 / 64.0;

fn err(layer: &'static str) -> impl Fn(String) -> BlocksError {
    move |message| BlocksError::Overlay { layer, message }
}

/// The Thornby plan for `seed`.
pub fn thornby_plan(seed: u64) -> Result<TownPlan, BlocksError> {
    let (site, terrain) = arda_town::samples::thornby();
    arda_town::generate(&site, &terrain, seed).map_err(|e| err("town")(e.to_string()))
}

/// The square holding the plan's market centre, in the demo frame.
pub fn market_square(plan: &TownPlan) -> (i64, i64) {
    arda_town::plan::square::square_of(plan.focal.market)
}

/// Metres of the north-west corner of demo square `(x, y)`.
#[allow(clippy::cast_precision_loss)] // demo frames stay far below 2^52 squares
fn metres(x: i64, y: i64) -> [f64; 2] {
    [x as f64 * SQUARE_M, y as f64 * SQUARE_M]
}

const fn ways_class(c: arda_town::site::RoadClass) -> arda_ways::RoadClass {
    use arda_town::site::RoadClass as T;
    use arda_ways::RoadClass as W;
    match c {
        T::None => W::None,
        T::Track => W::Track,
        T::Road => W::Road,
        T::Highway => W::Highway,
        T::Footpath => W::Footpath,
    }
}

/// Thornby's entering roads laid by arda-ways over the refined window.
pub fn ways(ctx: &OverlayCtx<'_>, off: (i64, i64)) -> Result<OverlayLayer, BlocksError> {
    let (_, terrain) = arda_town::samples::thornby();
    #[allow(clippy::cast_possible_truncation)] // sample metres are small
    let roads: Vec<arda_ways::Road> = terrain
        .roads
        .iter()
        .enumerate()
        .map(|(i, r)| arda_ways::Road {
            id: u64::try_from(i + 1).unwrap_or(u64::MAX),
            class: ways_class(r.class),
            segments: vec![r
                .points
                .iter()
                .map(|p| [p.x.round() as i64, p.y.round() as i64])
                .collect()],
            wealth: 128,
        })
        .collect();
    let flat = arda_ways::FnTerrain {
        height: |_: f64, _: f64| 0.0,
        channels: Vec::new(),
    };
    let mut layout = ctx.base.clone();
    let origin = metres(ctx.gsx0 + off.0, ctx.gsy0 + off.1);
    let out = arda_ways::apply_ways(&mut layout, origin, &roads, &[], &flat, ctx.seed)
        .map_err(|e| err("ways")(e.to_string()))?;
    let _ = arda_ways::fallback::degrade(&mut layout, ctx.library);
    let owned = out
        .sidecar
        .squares
        .iter()
        .map(|s| s.feature != arda_ways::Feature::None)
        .collect();
    Ok(OverlayLayer {
        layout,
        rules: out.rules,
        owned,
        soft: Vec::new(),
        elevation: false,
        review: Vec::new(),
    })
}

/// arda-fields' `village_strips` countryside, its village on the market.
pub fn fields(
    ctx: &OverlayCtx<'_>,
    off: (i64, i64),
    market: (i64, i64),
) -> Result<OverlayLayer, BlocksError> {
    let s = arda_fields::synthetic::village_strips();
    let village = s.settlements.first().map_or((0.0, 0.0), |v| (v.x_m, v.y_m));
    #[allow(clippy::cast_possible_truncation)] // sample metres are small
    let (vx, vy) = (
        (village.0 / SQUARE_M).floor() as i64,
        (village.1 / SQUARE_M).floor() as i64,
    );
    let (fx, fy) = (
        ctx.gsx0 + off.0 - market.0 + vx,
        ctx.gsy0 + off.1 - market.1 + vy,
    );
    let win = arda_fields::generate(&s.inputs(), metres(fx, fy), ctx.width, ctx.height, ctx.seed)
        .map_err(|e| err("fields")(e.to_string()))?;
    let (layout, _) = arda_fields::degrade::adapt(&win.layout, ctx.library);
    Ok(OverlayLayer {
        layout: relabel(layout, ctx),
        rules: win.rules,
        owned: win.owned,
        soft: Vec::new(),
        elevation: false,
        review: Vec::new(),
    })
}

/// The Thornby plan cut to the window.
pub fn town(
    ctx: &OverlayCtx<'_>,
    plan: &TownPlan,
    off: (i64, i64),
) -> Result<OverlayLayer, BlocksError> {
    let win = arda_town::block::Window {
        x: ctx.gsx0 + off.0,
        y: ctx.gsy0 + off.1,
        w: i64::from(ctx.width),
        h: i64::from(ctx.height),
    };
    let blk = arda_town::block::generate(plan, win).map_err(|e| err("town")(e.to_string()))?;
    let res = arda_town::block::fallback::resolve(&blk, ctx.library, plan.seed);
    Ok(OverlayLayer {
        layout: relabel(res.layout, ctx),
        rules: blk.to_rules(),
        owned: blk.owned.clone(),
        soft: Vec::new(),
        elevation: false,
        review: Vec::new(),
    })
}

/// Drops the layer's own frame origin; the composition keeps the base's.
fn relabel(mut layout: TacticalLayout, ctx: &OverlayCtx<'_>) -> TacticalLayout {
    layout.origin = ctx.base.origin;
    layout
}
