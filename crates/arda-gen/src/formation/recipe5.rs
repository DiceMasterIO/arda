//! Recipe-5 finishing stages, kept exactly as v0.1 ran them so recipe-5
//! worlds regenerate byte for byte (logic/02 §fine-formation recipes):
//! estuarine infill and the old river-mouth deltas, the shore rework, the
//! four-connected drainage guarantees and the glacial trough lakes. No
//! shore layer and no water forms are produced.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use rayon::prelude::*;

use super::lattice::{alloc, Lattice};
use super::macro_view::MacroView;
use super::{
    coast, drainage, glacial, sampled, surface, FormationError, LOWSTAND_MM, PREPARED_SPACING_UM,
    RELIEF_FULL_M, TROUGH_SINK_RADIUS_UM,
};

/// Finishes the formed lattice `g` as recipe 5 did.
///
/// # Errors
/// Allocation failure.
pub(super) fn finish(
    mut g: Lattice,
    view: &MacroView<'_>,
    base_seed: u64,
    is_basin: &(dyn Fn(i64, i64) -> bool + Sync),
) -> Result<Lattice, FormationError> {
    // logic/02 §fine-formation drowned coasts: estuarine infill beyond the
    // relief-dependent ria reach, then deltas at large low river mouths.
    {
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
        coast::deltas_v5(&mut g, &relief_q8, base_seed ^ 0xDE17A)?;
    }
    coast::rework_shore(&mut g, -LOWSTAND_MM)?;
    // Final drainage (logic/02 §fine-formation sampled drainage): every fine
    // land cell drains to the sea, and so does the 100 m point-sampled bed,
    // which runs last as the published authority.
    let fine_fill = |g: &mut Lattice,
                     is_sink: &(dyn Fn(i64, i64) -> bool + Sync)|
     -> Result<(), FormationError> {
        let flags = surface::sea_and_sinks(g, is_sink)?;
        let (w, h, n) = (g.width, g.height, g.z.len());
        let mut next: Vec<u32> = alloc(n)?;
        let mut closed: Vec<u8> = alloc(n)?;
        drainage::fill(&mut g.z, w, h, &flags, 1, &mut next, &mut closed)
    };
    fine_fill(&mut g, is_basin)?;
    sampled::drain_sampled_v5(&mut g, PREPARED_SPACING_UM, is_basin)?;
    // logic/02 §fine-formation glacial lakes: deliberately closed trough
    // basins; the guarantees run again with each trough's sink protected.
    let troughs = glacial::trough_lakes(&mut g)?;
    if !troughs.is_empty() {
        let is_sink = |x_um: i64, y_um: i64| {
            is_basin(x_um, y_um)
                || troughs.iter().any(|&(sx, sy)| {
                    let (dx, dy) = (i128::from(x_um - sx), i128::from(y_um - sy));
                    dx * dx + dy * dy <= TROUGH_SINK_RADIUS_UM * TROUGH_SINK_RADIUS_UM
                })
        };
        fine_fill(&mut g, &is_sink)?;
        sampled::drain_sampled_v5(&mut g, PREPARED_SPACING_UM, &is_sink)?;
    }
    Ok(g)
}
