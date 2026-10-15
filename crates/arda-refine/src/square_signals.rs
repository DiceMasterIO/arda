//! Square-scale corrections of the cell signals (recipe 9, review gallery
//! 2026-10-15): the world decides forest and beaches per 100 m cell, but a
//! battle map sees each 5-ft square, so two cell signals are brought down to
//! the square here.
//!
//! - **Held slope.** The fine terrain can run much steeper than the cell's
//!   own slope (MICRO 42 cell 716,1423: 27° in the world, 45–57° over a
//!   third of its squares). The world grew its forest at the cell's slope,
//!   so under that forest the steep-ground rules (bare-rock exposure, cliff
//!   faces) read a slope held to what the world itself expects of the
//!   cell's steepest squares: its slope plus
//!   [`arda_core::biome::ground::CRAG_LEAD_MILLI`]. Open ground keeps the
//!   square's slope.
//! - **Strand.** The coast's shoreline signal (sand beach, shingle, salt
//!   marsh, rocky shore) reaches up to three cells inland at cell scale; on
//!   the map it applies only on the strand, the band of squares the sea's
//!   standing-water indicator reaches, so sand is a beach at the water and
//!   not whole cells of a coastal town.
//!
//! - **Clearings.** Trees grow in groves cut from a slow noise field
//!   ([`crate::density::woods`]), but the ground followed the cell's cover
//!   alone, so a forest cell's glades showed bare forest floor with no
//!   tree on it. The ground prior now reads the same grove field: litter
//!   gives way to grass where no canopy stands.
//!
//! Older recipes keep the cell signals as they were.

use crate::classes::Class;
use crate::context::Ctx;
use crate::noise::smoothstep;
use crate::terrain::{Phys, Water};

/// Forest density over which the canopy goes from not holding steep ground
/// to holding it fully.
pub const HOLD_FD: (f64, f64) = (0.3, 0.7);

/// The sea's standing-water indicator over which a square goes from inland
/// to strand. Trunks stand only inland of it, so no tree grows on the
/// strand's sand; the shore rule fixes sand from -0.3 seaward.
pub const STRAND_SV: (f64, f64) = (-0.5, -0.36);

/// How far inland of the strand trees keep, in standing-water indicator
/// units (a few squares): a square's ground follows its corners, which
/// also read the squares around it.
pub const TRUNK_MARGIN: f64 = 0.08;

/// What a vertex's own squares tell the ground prior about the cells'
/// signals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Local {
    /// How far the vertex lies on the sea's strand ([`strand`]).
    pub strand: f64,
    /// Expected canopy over the vertex ([`crate::density::Woods::canopy`]).
    pub canopy: f64,
}

impl Local {
    /// The cell signals unchanged: on the strand, under full canopy.
    pub const CELL: Self = Self {
        strand: 1.0,
        canopy: 1.0,
    };
}

/// Opens a glade in interpolated ground weights `w` under canopy `canopy`
/// (`0..=1`): forest floor and leaf litter give way to grass by as much
/// as the trees are missing. Full canopy leaves `w` unchanged.
pub fn clearing(w: &mut crate::prior::Weights, canopy: f64) {
    let open = 1.0 - canopy.clamp(0.0, 1.0);
    let (ff, ll) = (Class::ForestFloor.index(), Class::LeafLitter.index());
    let freed = w[ff] * 0.7 * open + w[ll] * 0.5 * open;
    w[ff] *= 1.0 - 0.7 * open;
    w[ll] *= 1.0 - 0.5 * open;
    // Grass, not meadow: grass meets forest floor directly, while a
    // meadow border needs a scrub or litter band (`classes::PAIRS`), which
    // drew pale rims round every grove.
    w[Class::Grass.index()] += freed;
}

/// Whether the world takes the square-scale corrections (recipe 9).
#[must_use]
pub fn applies(ctx: &Ctx) -> bool {
    ctx.frame().seasonal()
}

/// The slope (degrees) the steep-ground rules read at global square
/// position `(u, v)` over a square of slope `slope_deg`: under the world's
/// forest, held to the cells' slope plus the crag lead; elsewhere, and on
/// older recipes, the square's own.
#[must_use]
pub fn held_slope(ctx: &Ctx, u: f64, v: f64, slope_deg: f64) -> f64 {
    if !applies(ctx) {
        return slope_deg;
    }
    let fd = ctx.bilinear(u, v, |c, _| f64::from(c.forest_density) / 255.0);
    let hold = smoothstep(HOLD_FD.0, HOLD_FD.1, fd);
    if hold <= 0.0 {
        return slope_deg;
    }
    #[allow(clippy::cast_precision_loss)] // a small constant
    let lead = arda_core::biome::ground::CRAG_LEAD_MILLI as f64 / 1000.0;
    let cap = ctx.bilinear(u, v, |c, _| f64::from(c.slope_milli_deg) / 1000.0) + lead;
    slope_deg - hold * (slope_deg - cap).max(0.0)
}

/// How far square `p` lies on the sea's strand, `0..=1`: 1 in the sea and
/// on the beach band, 0 inland and wherever no sea is near. Always 1 on
/// older recipes, which apply the shoreline signal across whole cells.
#[must_use]
pub fn strand(ctx: &Ctx, p: &Phys) -> f64 {
    if !applies(ctx) {
        return 1.0;
    }
    match (p.water, p.stand_kind) {
        (Water::Sea, _) => 1.0,
        (Water::Dry, Water::Sea) => smoothstep(STRAND_SV.0, STRAND_SV.1, p.stand_v),
        _ => 0.0,
    }
}

/// Whether a trunk may stand at square `p`: off the strand by
/// [`TRUNK_MARGIN`]. Always on older recipes.
#[must_use]
pub fn off_strand(ctx: &Ctx, p: &Phys) -> bool {
    if !applies(ctx) {
        return true;
    }
    match (p.water, p.stand_kind) {
        (Water::Dry, Water::Sea) => p.stand_v < STRAND_SV.0 - TRUNK_MARGIN,
        (Water::Dry, _) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{CellKey, GridSource};
    use arda::{Cell, Cover, TerrainKind};
    use arda_core::biome::Frame;

    fn ctx(seasonal: bool, fd: u8) -> Ctx {
        let cell = Cell {
            terrain: TerrainKind::Land,
            cover: Cover::Forest,
            forest_density: fd,
            slope_milli_deg: 27_000,
            ..Cell::default()
        };
        let mut g = GridSource::new(1, 7, 7, cell);
        g.set_frame(Frame::new(35, 55, 7).with_seasons(seasonal));
        Ctx::gather(&g, CellKey::new(3, 3)).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_closed_canopy_holds_the_slope_to_the_cells_lead() {
        let c = ctx(true, 230);
        let (u, v) = (3.0 * 64.0 + 10.0, 3.0 * 64.0 + 10.0);
        assert!((held_slope(&c, u, v, 50.0) - 35.0).abs() < 1e-6);
        assert!((held_slope(&c, u, v, 30.0) - 30.0).abs() < 1e-9);
    }

    #[test]
    fn open_ground_and_older_recipes_keep_the_square_slope() {
        let (u, v) = (3.0 * 64.0 + 10.0, 3.0 * 64.0 + 10.0);
        assert!((held_slope(&ctx(true, 20), u, v, 50.0) - 50.0).abs() < 1e-9);
        assert!((held_slope(&ctx(false, 230), u, v, 50.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn the_strand_is_the_sea_band_only() {
        let c = ctx(true, 0);
        let mut p = Phys::default();
        assert!(strand(&c, &p) < 1e-9, "no sea near");
        p.stand_kind = Water::Sea;
        p.stand_v = -0.55;
        assert!(strand(&c, &p) < 1e-9, "inland of the strand");
        p.stand_v = -0.3;
        assert!((strand(&c, &p) - 1.0).abs() < 1e-9, "on the beach");
        p.stand_kind = Water::Lake;
        assert!(strand(&c, &p) < 1e-9, "a lake shore is no sea strand");
        assert!((strand(&ctx(false, 0), &Phys::default()) - 1.0).abs() < 1e-9);
    }
}
