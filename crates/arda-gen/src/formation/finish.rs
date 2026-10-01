//! Recipe-6 finishing stages after the level loop (logic/02
//! §fine-formation coast, terraces, bathymetry, littoral, canyons, flats,
//! shore classes and §world-water): the coast, then water, the drainage
//! guarantees, and the shore survey last so it describes the published
//! surface.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use rayon::prelude::*;

use super::lattice::{alloc, Lattice};
use super::macro_view::MacroView;
use super::{
    arid, bathymetry, canyon, coast, coastal, drainage, flats, flow, glacial, littoral, margin,
    sampled, shore, surface, terrace, water, FormationError, Formed, LOWSTAND_MM,
    PREPARED_SPACING_UM, RELIEF_FULL_M, SINK_RADIUS_UM, TROUGH_SINK_RADIUS_UM,
};

/// What the macro stages hand the recipe-6 finish.
pub(super) struct Context<'a> {
    pub view: &'a MacroView<'a>,
    pub base_seed: u64,
    /// Endorheic sink points (µm) of the macro basins.
    pub basins: &'a [(i64, i64)],
    pub is_basin: &'a (dyn Fn(i64, i64) -> bool + Sync),
    pub shelf: &'a bathymetry::Shelf,
    pub volcanoes: &'a [margin::Volcano],
    pub landforms: Vec<arda_core::Landform>,
    /// Arid endorheic basins (recipe 7; empty before).
    pub arid: &'a [arid::AridBasin],
}

/// Finishes the formed lattice `g` (logic/02 §fine-formation, recipe 6).
///
/// # Errors
/// Allocation failure.
pub(super) fn run(mut g: Lattice, ctx: Context<'_>) -> Result<Formed, FormationError> {
    let Context {
        view,
        base_seed,
        basins,
        is_basin,
        shelf,
        volcanoes,
        mut landforms,
        arid: arid_basins,
    } = ctx;
    // logic/02 §fine-formation drowned coasts: estuarine infill beyond the
    // relief-dependent ria reach.
    let relief_q8 = {
        let (w, d) = (g.width, g.spacing_um);
        let n = g.z.len();
        let mut macro_fine: Vec<i32> = alloc(n)?;
        let mut relief_q8: Vec<u8> = alloc(n)?;
        macro_fine
            .par_iter_mut()
            .zip(relief_q8.par_iter_mut())
            .enumerate()
            .for_each(|(i, (m, r))| {
                let (x_um, y_um) = ((i % w) as i64 * d, (i / w) as i64 * d);
                *m = view.height(x_um, y_um);
                let rel = view.relief_m(x_um, y_um).min(RELIEF_FULL_M);
                *r = (rel * 255 / RELIEF_FULL_M) as u8;
            });
        coast::infill(&mut g, &macro_fine, &relief_q8)?;
        drop(macro_fine);
        // logic/02 §fine-formation terraces: lowland terraces, bluffs and
        // flat interfluves on the infilled coastal lowland.
        terrace::apply(&mut g, &relief_q8, base_seed ^ 0x7E44_0000)?;
        // logic/02 §fine-formation bathymetry: passive shelves shallow
        // toward the margin profile, seen through the same macro warp.
        let shelf_view = MacroView {
            height: &shelf.target,
            relief: &shelf.activity,
            ..*view
        };
        let (mut target, mut activity): (Vec<i32>, Vec<u8>) = (alloc(n)?, alloc(n)?);
        target
            .par_iter_mut()
            .zip(activity.par_iter_mut())
            .enumerate()
            .for_each(|(i, (t, a))| {
                let (x_um, y_um) = ((i % w) as i64 * d, (i / w) as i64 * d);
                *t = shelf_view.height(x_um, y_um);
                *a = u8::try_from(shelf_view.relief_m(x_um, y_um).min(255)).unwrap_or(255);
            });
        bathymetry::shelf_fill(&mut g, &target, &activity)?;
        drop(target);
        drop(activity);
        relief_q8
    };
    coast::rework_shore(&mut g, -LOWSTAND_MM)?;
    // logic/02 §fine-formation littoral: cliffs, bay beaches, barriers;
    // then §canyons: submarine canyons off the major river mouths.
    let littoral = {
        let mouths = coastal::river_mouths(&g, &flow::route(&g)?);
        let setting = coastal::compute(&g, base_seed, &mouths)?;
        let built = littoral::apply(
            &mut g,
            &setting,
            &mouths,
            base_seed ^ 0x0117,
            base_seed,
            volcanoes,
        )?;
        drop(setting);
        canyon::carve(&mut g, &mouths)?;
        built
    };
    let fine_fill = |g: &mut Lattice,
                     is_basin: &(dyn Fn(i64, i64) -> bool + Sync)|
     -> Result<(), FormationError> {
        let flags = surface::sea_and_sinks(g, is_basin)?;
        let (w, h, n) = (g.width, g.height, g.z.len());
        let mut next: Vec<u32> = alloc(n)?;
        let mut closed: Vec<u8> = alloc(n)?;
        drainage::fill(&mut g.z, w, h, &flags, 1, &mut next, &mut closed)
    };
    // logic/02 §fine-formation flats: fill flats (sediment-filled basins,
    // estuarine infill) are regraded as cost-weighted geodesics, so rivers
    // wander across them instead of running along grid geodesics.
    fine_fill(&mut g, is_basin)?;
    {
        let flags = surface::sea_and_sinks(&g, is_basin)?;
        flats::regrade(&mut g, &flags, base_seed ^ 0xF1A7_5000)?;
    }
    // logic/02 §world-water deltas: the one delta rule. River-led,
    // volume-limited lobes, split into delta islands on large rivers. Built
    // after the shore rework and the littoral headland retreat, which would
    // otherwise smooth the lobes away or cut them back into cliffs.
    let mut features = water::WaterFeatures::default();
    // logic/02 §fine-formation climate runoff (recipe 7): channels are
    // sized from runoff-weighted area.
    let runoff = match view.water {
        Some(_) => Some(arid::weights(view, g.width, g.height, g.spacing_um)?),
        None => None,
    };
    // The macro surface is sampled again here rather than held through the
    // littoral stages, to keep the peak allocation down.
    let mut macro_fine: Vec<i32> = alloc(g.z.len())?;
    {
        let (w, d) = (g.width, g.spacing_um);
        macro_fine.par_iter_mut().enumerate().for_each(|(i, m)| {
            *m = view.height((i % w) as i64 * d, (i / w) as i64 * d);
        });
    }
    water::delta::build(
        &mut g,
        &macro_fine,
        &relief_q8,
        base_seed ^ 0xDE17A,
        &mut features,
        runoff.as_deref(),
    )?;
    drop(macro_fine);
    // Final drainage (logic/02 §fine-formation sampled drainage): every fine
    // land cell drains to the sea, and so does the 100 m point-sampled bed.
    // The sampled pass runs last: the published 100 m bed is the authority.
    // Raising its support nodes can leave a few isolated fine pits, which
    // no downstream stage reads.
    fine_fill(&mut g, is_basin)?;
    // logic/02 §world-water arid basins (recipe 7): sediment-filled playas
    // around the sinks of basins whose lakes can never spill. Built on the
    // drained surface, where a flood from the sink rises monotonically to
    // the spill, and drained again before the channels are shaped.
    if !arid_basins.is_empty() {
        arid::carve_pans(
            &mut g,
            arid_basins,
            (SINK_RADIUS_UM as i64, PREPARED_SPACING_UM),
            &mut features,
        )?;
        fine_fill(&mut g, is_basin)?;
        let flags = surface::sea_and_sinks(&g, is_basin)?;
        flats::regrade(&mut g, &flags, base_seed ^ 0xF1A7_5007)?;
    }
    // logic/02 §world-water: braided belts and meanders, drained again,
    // then oxbows and karst poljes; their closed basins become protected
    // sinks for every later drainage guarantee.
    let shaped = water::shape_channels(
        &mut g,
        &relief_q8,
        base_seed ^ 0x3A7E_5000,
        &mut features,
        runoff.as_deref(),
    )?;
    drop(relief_q8);
    fine_fill(&mut g, is_basin)?;
    water::shape_basins(&mut g, &shaped, &mut features, runoff.as_deref())?;
    drop(shaped);
    drop(runoff);
    features.index();
    let is_shaped = |x_um: i64, y_um: i64| is_basin(x_um, y_um) || features.is_sink(x_um, y_um);
    fine_fill(&mut g, &is_shaped)?;
    sampled::drain_sampled(&mut g, PREPARED_SPACING_UM, &is_shaped)?;
    // logic/02 §fine-formation glacial lakes: deliberately closed trough
    // basins. Overlapping carve discs also leave secondary hollows, so the
    // drainage guarantees run again with each trough's sink protected.
    let troughs = glacial::trough_lakes(&mut g)?;
    // The guarantees run once more, with each trough's sink protected: the
    // fine fill also grades the pockets the sampled pass levelled, which it
    // leaves exactly flat, and the last sampled pass keeps the published
    // 100 m bed the authority.
    let is_sink = |x_um: i64, y_um: i64| {
        is_shaped(x_um, y_um)
            || troughs.iter().any(|&(sx, sy)| {
                let (dx, dy) = (i128::from(x_um - sx), i128::from(y_um - sy));
                dx * dx + dy * dy <= TROUGH_SINK_RADIUS_UM * TROUGH_SINK_RADIUS_UM
            })
    };
    fine_fill(&mut g, &is_sink)?;
    sampled::drain_sampled(&mut g, PREPARED_SPACING_UM, &is_sink)?;
    water::record_basins(
        &mut features,
        basins,
        &troughs,
        SINK_RADIUS_UM,
        TROUGH_SINK_RADIUS_UM,
    );
    if !arid_basins.is_empty() {
        for s in &mut features.sinks {
            if s.kind == water::SinkKind::Tectonic
                && arid_basins.iter().any(|b| b.sink == (s.x_um, s.y_um))
            {
                s.kind = water::SinkKind::AridTerminal;
            }
        }
        features.index();
    }
    // logic/02 §fine-formation shore: classes and island census, last, so
    // they describe the published surface.
    let built = shore::Builders {
        volcanoes,
        deltas: &features.deltas,
        barrier_cells: &littoral.barrier_cells,
    };
    let mut layer = shore::survey(&g, PREPARED_SPACING_UM, base_seed, &built)?;
    landforms.extend(troughs.iter().map(|&(x_um, y_um)| arda_core::Landform {
        x_um,
        y_um,
        area_km2: 0,
        relief_m: 0,
        kind: arda_core::LandformKind::GlacialTrough,
        cause: arda_core::LandformCause::Glacial,
    }));
    layer.landforms = landforms;
    Ok(Formed {
        lattice: g,
        shore: Some(layer),
        water: Some(features),
    })
}
