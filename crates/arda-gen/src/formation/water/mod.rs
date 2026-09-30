//! Rivers and lakes shaped into the formed surface (logic/02 §world-water,
//! goals 8-13).
//!
//! After the first final drainage fill, the main-stem network of the fine
//! lattice is extracted and shaped in two phases, with a drainage fill
//! between them ([`shape_channels`], then [`shape_basins`]):
//! - braided belts where slope exceeds the Leopold–Wolman threshold, bed
//!   load is plentiful (piedmont or proglacial) and the valley opens;
//! - free meanders on alluvial floors below that threshold, with oxbow
//!   cutoffs;
//! - karst poljes on a carbonate proxy, and dolines recorded below the
//!   100 m scale.
//!
//! Deltas are built earlier, at the coast ([`delta`]). Every closed basin
//! made here is a protected sink; the annual water balance decides whether
//! it holds a lake. The passes only reshape the bed: water is still routed
//! and balanced exactly by the shared hydrology.

pub mod braid;
pub mod delta;
pub mod delta_net;
pub mod geom;
pub mod karst;
pub mod meander;
pub mod network;
pub mod oxbow;

use arda_core::DischargeMilli;

use super::lattice::Lattice;
use super::FormationError;

/// Runoff assumed when sizing channels during formation, before climate
/// exists, millimetres per year.
pub const NOMINAL_RUNOFF_MM: u64 = 500;
/// Streams below this catchment are not shaped, km².
pub const STREAM_MIN_KM2: u64 = 3;

/// Why a protected closed basin exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SinkKind {
    /// Endorheic tectonic basin.
    Tectonic,
    /// Overdeepened glacial trough.
    Glacial,
    /// Meander cutoff.
    Oxbow,
    /// Karst polje.
    Karst,
}

/// A protected closed-basin sink: a disc kept at its carved depth by every
/// later drainage guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sink {
    /// Centre, absolute micrometres.
    pub x_um: i64,
    /// Centre, absolute micrometres.
    pub y_um: i64,
    /// Protected radius, micrometres.
    pub radius_um: i64,
    /// Origin.
    pub kind: SinkKind,
}

/// A river-mouth delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delta {
    /// Apex, absolute micrometres.
    pub apex_um: (i64, i64),
    /// Fan radius, metres.
    pub radius_m: i64,
    /// Feeding catchment, km².
    pub catchment_km2: i64,
}

/// A karst doline below the 100 m cell scale (recorded, not carved: the
/// shared hydrology has no groundwater to keep it dry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DolineSite {
    /// Centre, absolute micrometres.
    pub x_um: i64,
    /// Centre, absolute micrometres.
    pub y_um: i64,
    /// Rim radius, millimetres.
    pub radius_mm: i64,
    /// Depth, millimetres.
    pub depth_mm: i64,
}

/// Counters for metrics and tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WaterStats {
    /// Meandering spans carved.
    pub meander_spans: u64,
    /// Valley length of those spans, Q8 cells.
    pub meander_valley_q8: i64,
    /// Course length of those spans, Q8 cells.
    pub meander_course_q8: i64,
    /// Oxbow crescents carved.
    pub oxbows: u64,
    /// Braided reaches built.
    pub braided_reaches: u64,
    /// Braided valley length, Q8 cells.
    pub braided_q8: i64,
    /// Karst poljes carved.
    pub poljes: u64,
    /// Share of land on the carbonate proxy, ‰.
    pub karst_permille: u64,
    /// River mouths with at least the delta catchment, on any coast.
    pub delta_mouths: u64,
    /// Mouths that the superseded 2,000 km² rule would have considered.
    pub delta_old_rule_mouths: u64,
    /// Deltas built.
    pub deltas: u64,
    /// Deltas split into islands by tidal channels.
    pub delta_islands: u64,
}

/// Everything formation records about its rivers and lakes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WaterFeatures {
    /// Protected closed-basin sinks, with origin.
    pub sinks: Vec<Sink>,
    /// Braided belt: 100 m cells `(x, y)` on the braided course with the
    /// belt width in metres.
    pub braided_cells: Vec<(u32, u32, u32)>,
    /// River-mouth deltas.
    pub deltas: Vec<Delta>,
    /// Karst dolines.
    pub dolines: Vec<DolineSite>,
    /// Counters.
    pub stats: WaterStats,
}

impl WaterFeatures {
    /// Sorted lookup for [`Self::is_sink`]; call after the last sink is added.
    pub fn index(&mut self) {
        self.sinks.sort_by_key(|s| {
            (
                s.x_um.div_euclid(BUCKET_UM),
                s.y_um.div_euclid(BUCKET_UM),
                s.y_um,
                s.x_um,
                s.kind,
            )
        });
        self.braided_cells.sort_unstable();
        self.braided_cells.dedup_by_key(|c| (c.0, c.1));
    }

    /// Whether a point lies in a protected sink disc. Requires [`Self::index`].
    /// Sinks are bucketed on a 2 km grid, so a lookup reads only the nine
    /// buckets around the point (an oxbow is protected as a chain of discs
    /// along its whole crescent).
    #[must_use]
    pub fn is_sink(&self, x_um: i64, y_um: i64) -> bool {
        let (bx, by) = (x_um.div_euclid(BUCKET_UM), y_um.div_euclid(BUCKET_UM));
        let key = |s: &Sink| (s.x_um.div_euclid(BUCKET_UM), s.y_um.div_euclid(BUCKET_UM));
        (bx - 1..=bx + 1).any(|cx| {
            let lo = self.sinks.partition_point(|s| key(s) < (cx, by - 1));
            self.sinks[lo..]
                .iter()
                .take_while(|s| key(s) <= (cx, by + 1))
                .any(|s| {
                    let (dx, dy) = (i128::from(x_um - s.x_um), i128::from(y_um - s.y_um));
                    dx * dx + dy * dy <= i128::from(s.radius_um) * i128::from(s.radius_um)
                })
        })
    }
}

/// Adds the tectonic basin and glacial trough sinks, which formation
/// protects itself, so every closed basin carries its origin; re-indexes.
pub fn record_basins(
    features: &mut WaterFeatures,
    basins: &[(i64, i64)],
    troughs: &[(i64, i64)],
    basin_radius_um: i128,
    trough_radius_um: i128,
) {
    for (list, radius, kind) in [
        (basins, basin_radius_um, SinkKind::Tectonic),
        (troughs, trough_radius_um, SinkKind::Glacial),
    ] {
        for &(x_um, y_um) in list {
            features.sinks.push(Sink {
                x_um,
                y_um,
                radius_um: i64::try_from(radius).unwrap_or(i64::MAX),
                kind,
            });
        }
    }
    features.index();
}

/// Sink lookup bucket: wider than any protected radius.
const BUCKET_UM: i64 = 2_000_000_000;

/// Lattice cell area in whole square metres.
#[must_use]
pub fn cell_m2(g: &Lattice) -> u64 {
    let mm = u64::try_from(g.spacing_um / 1000).unwrap_or(0);
    (mm * mm / 1_000_000).max(1)
}

/// Mean discharge of a catchment at [`NOMINAL_RUNOFF_MM`].
#[must_use]
pub fn nominal_discharge(km2: u64) -> DischargeMilli {
    // km² × 1e6 m² × 0.5 m / 31,557,600 s, in L/s.
    DischargeMilli::new(km2 * NOMINAL_RUNOFF_MM * 1_000_000 / 31_557_600)
}

/// Channels shaped by [`shape_channels`], awaiting [`shape_basins`].
pub struct Shaped {
    streams: Vec<network::Stream>,
    area: Vec<u32>,
    claimed: Vec<Vec<bool>>,
    cutoffs: Vec<oxbow::Cutoff>,
    seed: u64,
}

/// First phase: braided belts and meanders are carved into a drained
/// lattice. `relief_q8` is the macro relief mask on the same lattice (255 =
/// mountainous). Drain the lattice again before [`shape_basins`], so pits
/// the channels leave never merge with a deliberate basin.
///
/// # Errors
/// Allocation failure.
pub fn shape_channels(
    g: &mut Lattice,
    relief_q8: &[u8],
    seed: u64,
    features: &mut WaterFeatures,
) -> Result<Shaped, FormationError> {
    let net = network::build(g)?;
    let min_cells = u32::try_from(STREAM_MIN_KM2 * 1_000_000 / cell_m2(g)).unwrap_or(u32::MAX);
    let smooth = usize::try_from(geom::m_to_q8(g, 400) / geom::CELL_Q8).unwrap_or(1);
    let mut streams = network::streams(g, &net, min_cells, smooth.max(1));
    // Largest rivers first, so tributaries adapt to the carved trunks.
    streams.sort_by_key(|s| {
        let last = s.cells[s.cells.len().saturating_sub(2)];
        (std::cmp::Reverse(net.area[last]), s.cells[0])
    });
    let mut claimed: Vec<Vec<bool>> = streams.iter().map(|s| vec![false; s.cells.len()]).collect();
    braid::apply(
        g,
        &streams,
        &net.area,
        relief_q8,
        &mut claimed,
        seed ^ 0xB4A1,
        features,
    );
    let cutoffs = meander::apply(
        g,
        &streams,
        &net.area,
        &mut claimed,
        seed ^ 0x3EA7,
        features,
    );
    Ok(Shaped {
        streams,
        area: net.area,
        claimed,
        cutoffs,
        seed,
    })
}

/// Second phase, on the re-drained lattice: oxbow crescents and karst
/// poljes (closed basins recorded as sinks) and dolines.
///
/// # Errors
/// Allocation failure.
pub fn shape_basins(
    g: &mut Lattice,
    shaped: &Shaped,
    features: &mut WaterFeatures,
) -> Result<(), FormationError> {
    oxbow::carve_cutoffs(g, &shaped.cutoffs, features)?;
    karst::apply(
        g,
        &shaped.streams,
        &shaped.area,
        &shaped.claimed,
        shaped.seed ^ 0x4A55,
        features,
    )
}

/// Both phases without the intermediate drainage (tests and tools).
///
/// # Errors
/// Allocation failure.
pub fn shape(
    g: &mut Lattice,
    relief_q8: &[u8],
    seed: u64,
    features: &mut WaterFeatures,
) -> Result<(), FormationError> {
    let shaped = shape_channels(g, relief_q8, seed, features)?;
    shape_basins(g, &shaped, features)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn sink_lookup_finds_only_points_inside_discs() {
        let mut f = WaterFeatures::default();
        for (x, kind) in [
            (5_000_000_000, SinkKind::Oxbow),
            (9_000_000_000, SinkKind::Karst),
        ] {
            f.sinks.push(Sink {
                x_um: x,
                y_um: 1_000_000_000,
                radius_um: 75_000_000,
                kind,
            });
        }
        f.index();
        assert!(f.is_sink(5_000_000_000, 1_050_000_000));
        assert!(!f.is_sink(5_000_000_000, 1_080_000_000));
        assert!(f.is_sink(9_070_000_000, 1_000_000_000));
        assert!(!f.is_sink(7_000_000_000, 1_000_000_000));
    }

    #[test]
    fn nominal_discharge_scales_with_catchment() {
        // 100 km² at 500 mm/yr: 1.584 m³/s.
        assert_eq!(nominal_discharge(100).raw(), 1_584);
    }
}
