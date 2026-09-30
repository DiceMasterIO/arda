//! Public close-zoom relief entry points on [`AtlasTerrain`]
//! (logic/17 §render): colour one pixel from refined geometry and place
//! saved river vertices exactly where the overview draws them.

use super::{fine, invalid, AtlasTerrain};
use crate::RenderError;
use arda_core::GlobalCell;

pub use fine::ReliefGeometry;

impl AtlasTerrain {
    /// Formed-overview colour of one point at absolute fine-lattice
    /// micrometres (world position minus `arda_core::FINE_FRAME_OFFSET_UM`),
    /// lit with caller-supplied refined geometry. The point must lie in this
    /// area's saved cells (or their one-cell halo); `footprint_um` is the
    /// pixel size, which fades texture before it aliases. The flag is true
    /// on land (false on sea and lake water).
    ///
    /// # Errors
    /// Not a recipe-5 formed area, or the point lies outside its context.
    pub fn relief_colour(
        &self,
        x_um: i64,
        y_um: i64,
        footprint_um: i64,
        geometry: ReliefGeometry,
    ) -> Result<([u8; 3], bool), RenderError> {
        let fine = self
            .fine
            .as_ref()
            .filter(|f| f.is_formed())
            .ok_or_else(|| invalid("relief shading requires recipe-5 formed terrain"))?;
        fine.relief_colour(
            i128::from(x_um),
            i128::from(y_um),
            i128::from(footprint_um.max(1)),
            geometry,
            &fine::SavedFields {
                classes: &self.classes,
                wetness: &self.wetness,
                moisture: &self.moisture,
                forest_density: &self.forest_density,
                temperature: &self.temperature,
                heights: &self.heights,
                lake_surface: &self.lake_surface,
                shore: &self.shore,
            },
        )
    }

    /// Where the overview draws a saved channel vertex: the cell centre plus
    /// the recipe-5 thalweg and meander offsets, in fine-lattice micrometres.
    #[must_use]
    pub fn river_vertex_um(&self, node: GlobalCell) -> (i64, i64) {
        let (ox, oy) = self.channel_offset_um(node);
        (
            i64::from(node.x) * 100_000_000 + ox,
            i64::from(node.y) * 100_000_000 + oy,
        )
    }
}

/// Recipe-5 river water colour for a saved discharge (goal 28), the same
/// blue-teal the formed overview uses (`formed_river_colour` in the channel
/// overlay); `None` below the drawn threshold.
#[must_use]
pub fn formed_river_rgb(discharge_milli: u64) -> Option<[u8; 3]> {
    crate::carto::river_band(discharge_milli).map(crate::overview::formed_river_colour)
}

/// The formed overview's river chain rules (logic/04 §atlas-formed rivers)
/// for close-zoom relief: main-stem neighbours, sources, the one-step
/// relaxation of D8 staircases and the uniform cubic B-spline centreline,
/// so relief rivers follow the same curves as the overview's.
pub struct FormedRiverNetwork(crate::channel_curve::Network);

impl FormedRiverNetwork {
    /// Indexes directed saved edges `(from, to, discharge)`.
    pub fn new(edges: impl IntoIterator<Item = (GlobalCell, GlobalCell, u64)>) -> Self {
        Self(crate::channel_curve::Network::new(edges))
    }

    /// Whether no channel flows into `node` (a stream starts there and
    /// tapers).
    #[must_use]
    pub fn is_source(&self, node: GlobalCell) -> bool {
        self.0.is_source(node)
    }

    /// Upstream and downstream tangent neighbours of the edge `from -> to`.
    #[must_use]
    pub fn neighbours(
        &self,
        from: GlobalCell,
        to: GlobalCell,
    ) -> (Option<GlobalCell>, Option<GlobalCell>) {
        self.0.neighbours(from, to)
    }

    /// Main-stem upstream (largest inflow) and downstream (largest
    /// outflow) nodes of `node`, the ones [`Self::relaxed`] reads.
    #[must_use]
    pub fn main_stem(&self, node: GlobalCell) -> (Option<GlobalCell>, Option<GlobalCell>) {
        self.0.main_stem(node)
    }

    /// The overview's relaxed position of `node`: `(prev + 2 p + next) / 4`
    /// along its main stem, where `at` gives unrelaxed vertices.
    #[must_use]
    pub fn relaxed(
        &self,
        node: GlobalCell,
        at: impl Fn(GlobalCell) -> Option<(i64, i64)>,
    ) -> Option<(i64, i64)> {
        let p = at(node)?;
        Some(self.0.relaxed(node, p, at))
    }
}

/// `segments + 1` points of the formed river centreline from `a` to `b`
/// (a uniform cubic B-spline over the chain; a missing neighbour is
/// mirrored so the curve ends on its node), in the inputs' units.
#[must_use]
pub fn formed_river_centreline(
    before: Option<(i64, i64)>,
    a: (i64, i64),
    b: (i64, i64),
    after: Option<(i64, i64)>,
    segments: u32,
) -> Vec<(i64, i64)> {
    let wide = |p: (i64, i64)| (i128::from(p.0), i128::from(p.1));
    let (p1, p2) = (wide(a), wide(b));
    let p0 = before.map_or((2 * p1.0 - p2.0, 2 * p1.1 - p2.1), wide);
    let p3 = after.map_or((2 * p2.0 - p1.0, 2 * p2.1 - p1.1), wide);
    let n = i128::from(segments.max(1));
    (0..=n)
        .map(|i| {
            let ((x, y), _) = crate::channel_curve::bspline_at([p0, p1, p2, p3], i, n);
            (i64::try_from(x).unwrap_or(0), i64::try_from(y).unwrap_or(0))
        })
        .collect()
}

/// Width at a stream's source relative to its saved width (the overview's
/// taper, goal 28).
#[must_use]
pub fn formed_source_width(width: i64) -> i64 {
    crate::channel_curve::source_width(width)
}
