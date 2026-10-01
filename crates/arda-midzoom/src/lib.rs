//! On-demand mid-zoom relief for arda worlds (logic/17).
//!
//! The world stores one height every 39.0625 m; zoomed in, that is smooth
//! by construction. Storing a finer level everywhere would cost 4× the
//! memory, so — like the tactical layer — this crate grows ~10 m detail on
//! demand for the tiles on screen only:
//! 1. [`refine`]: the stored field as a sharpened spline base plus
//!    drainage-aligned gullies and spurs ([`detail`]), scaled by slope,
//!    relief, roughness and cover, keeping every 39 m cell mean and every
//!    saved river and lake where it is;
//! 2. [`tile`]: relief tiles shaded by the formed Atlas shader, continuing
//!    the overview pyramid past native resolution, with rivers, lakes,
//!    coasts and pools from the tactical layer's water geometry
//!    ([`water`]), so both zooms put every shore in the same place.
//!
//! Every value is integer and a pure function of the seed and the global
//! position, so tiles are deterministic and join pixel-exactly.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod base;
pub mod detail;
pub mod error;
pub mod fixed;
pub mod height;
pub mod lattice;
pub mod masks;
pub mod pyramid;
pub mod refine;
pub mod source;
pub mod tile;
pub mod water;
pub mod world;

pub use error::MidzoomError;
pub use height::{HeightTile, SurfacePoint};
pub use lattice::{world_lattice, Lattice};
pub use pyramid::Pyramid;
pub use refine::{refine_nodes, refine_nodes_for};
pub use source::{CellInfo, GridTerrain, Terrain, WorldTerrain, FINE_UM};
pub use tile::{render_tile, render_window, Rgba};
pub use water::{water_mask, WindowWater};
pub use world::ReliefWorld;

/// Refines the square world window `[x0_m, x0_m + size_m)²` (world metres,
/// north-west origin) at about `spacing_m` (snapped to 39.0625 m / n,
/// n ∈ {1, 2, 4, 8}); the returned nodes cover the window.
///
/// # Errors
/// An empty or oversized window, or a read failure.
pub fn refine_tile(
    world: &dyn Terrain,
    x0_m: f64,
    y0_m: f64,
    size_m: f64,
    spacing_m: f64,
) -> Result<HeightTile, MidzoomError> {
    if !(size_m > 0.0 && spacing_m > 0.0 && x0_m.is_finite() && y0_m.is_finite()) {
        return Err(MidzoomError::Window("non-positive size or spacing".into()));
    }
    let n = [8_i64, 4, 2, 1]
        .into_iter()
        .find(|&n| 39.0625 / n as f64 >= spacing_m * 0.99)
        .unwrap_or(1);
    let s = 39.0625 / n as f64;
    // Fine-lattice frame: world minus the 50 m I1 offset.
    let lx = x0_m - 50.0;
    let ly = y0_m - 50.0;
    #[allow(clippy::cast_possible_truncation)]
    let (i0, j0, i1, j1) = (
        (lx / s).floor() as i64,
        (ly / s).floor() as i64,
        ((lx + size_m) / s).ceil() as i64,
        ((ly + size_m) / s).ceil() as i64,
    );
    let w = usize::try_from(i1 - i0 + 1).map_err(|_| MidzoomError::Window("width".into()))?;
    let h = usize::try_from(j1 - j0 + 1).map_err(|_| MidzoomError::Window("height".into()))?;
    refine_nodes(world, i0, j0, w, h, n)
}
