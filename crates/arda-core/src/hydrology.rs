//! Format-4 shared hydrology identities, physical geometry and annual water records.
//!
//! `GlobalCell` identifies fine-grid positions; `DischargeMilli` stores u64 raw L/s.
//! Codecs write fields explicitly; Rust struct padding is never the format.
#![deny(missing_docs)]

use crate::{AreaCoord, DischargeMilli, GlobalCell, HeightMm};

macro_rules! identifier {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(
            /// Canonical raw identity in the type-specific namespace.
            pub u64,
        );
    };
}
identifier!(
    TerminalId,
    "Stable identity of a physical routing terminal."
);
identifier!(
    BasinId,
    "Stable identity of a coalesced physical basin node."
);
identifier!(
    ReachId,
    "Directed reach identity, with bit 63 reserved for physical terminals without a step."
);
identifier!(
    JunctionId,
    "Physical dry junction identity: packed global (y,x), without a separate table."
);

impl JunctionId {
    /// Packs an actual modeled junction coordinate; the caller validates its domain.
    #[must_use]
    pub fn at(cell: GlobalCell) -> Self {
        Self((u64::from(cell.y) << 32) | u64::from(cell.x))
    }
    /// Returns the exact coordinate named by this junction.
    #[must_use]
    pub fn cell(self) -> GlobalCell {
        let [a, b, c, d, e, f, g, h] = self.0.to_le_bytes();
        GlobalCell {
            x: u32::from_le_bytes([a, b, c, d]),
            y: u32::from_le_bytes([e, f, g, h]),
        }
    }
}
const POINT_REACH_BIT: u64 = 1 << 63;

impl ReachId {
    /// Packs the actual start and first D8 step, rejecting nonadjacency or overflow.
    /// Direction order is NW, N, NE, W, E, SW, S, SE. The caller validates domain membership.
    #[must_use]
    pub fn from_step(from: GlobalCell, to: GlobalCell) -> Option<Self> {
        let direction = match (
            i64::from(to.x) - i64::from(from.x),
            i64::from(to.y) - i64::from(from.y),
        ) {
            (-1, -1) => 0,
            (0, -1) => 1,
            (1, -1) => 2,
            (-1, 0) => 3,
            (1, 0) => 4,
            (-1, 1) => 5,
            (0, 1) => 6,
            (1, 1) => 7,
            _ => return None,
        };
        let raw = JunctionId::at(from)
            .0
            .checked_mul(8)?
            .checked_add(direction)?;
        (raw & POINT_REACH_BIT == 0).then_some(Self(raw))
    }
    /// Names a physical absorbing terminal or domain exit without inventing a step.
    /// Returns `None` if the packed coordinate collides with the reserved tag bit.
    #[must_use]
    pub fn point(at: GlobalCell) -> Option<Self> {
        let packed = JunctionId::at(at).0;
        (packed & POINT_REACH_BIT == 0).then_some(Self(POINT_REACH_BIT | packed))
    }
    /// Whether this record denotes a physical terminal with no directed cell step.
    #[must_use]
    pub fn is_point(self) -> bool {
        self.0 & POINT_REACH_BIT != 0
    }
    /// Decodes the actual start coordinate for either a course or a point record.
    #[must_use]
    pub fn start(self) -> GlobalCell {
        JunctionId(if self.is_point() {
            self.0 & !POINT_REACH_BIT
        } else {
            self.0 >> 3
        })
        .cell()
    }
}
identifier!(
    CatchmentId,
    "Stable identity of an immutable contributing catchment."
);

/// Exact nonnegative water volume; one millimetre over one 100 m cell is 10,000 L.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Litres(
    /// Whole litres; arithmetic and conversion into discharge are checked.
    pub u128,
);

/// Physical channel width in decimetres, using the same integer floor in generation and rendering.
/// The input is whole litres per second. Returns `None` when the width exceeds `u32`.
#[must_use]
pub fn channel_width_dm(discharge: DischargeMilli) -> Option<u32> {
    let n = u128::from(discharge.raw()).checked_mul(1000)?;
    let mut root = n;
    let mut next = n.div_ceil(2);
    while next < root {
        root = next;
        next = root.checked_add(n / root)? / 2;
    }
    u32::try_from(root.checked_mul(40)? / 1000).ok()
}

/// Domain union includes requested fringe and any existing exported overshoot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HydrologyDomain {
    /// Modeled width in 100 m cells, including hydrology-only fringe.
    pub width_cells: u32,
    /// Modeled height in 100 m cells, including hydrology-only fringe.
    pub height_cells: u32,
    /// Number of complete 512-cell area columns with final output files.
    pub exported_areas_wide: u32,
    /// Number of complete 512-cell area rows with final output files.
    pub exported_areas_high: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Canonical crossing pair uses GlobalCell derived Ord (x, then y).
/// This is deliberately distinct from terminal identity order (y, then x).
pub struct CrossingId {
    /// Smaller endpoint under the coordinate ordering `(x, y)`.
    pub low: GlobalCell,
    /// Larger endpoint under the coordinate ordering `(x, y)`.
    pub high: GlobalCell,
}

/// Immediate downstream destination of a directed water transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivingAccount {
    /// The next modeled river reach.
    Reach(ReachId),
    /// The receiving physical basin or connected lake component.
    Lake(BasinId),
    /// An actual dry junction; outgoing reaches may be absent in potential topology.
    Junction(JunctionId),
    /// Water entering the connected modeled ocean.
    Sea,
    /// Water leaving the actual outer modeled boundary.
    DomainExport,
}

/// Actual D8 neighboring cells witnessing a physical spill, oriented from the spilling source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpillConnection {
    /// Actual modeled cell on the source side of the physical spill.
    pub from: GlobalCell,
    /// None only for DomainExport at an actual modeled outer boundary.
    pub to: Option<GlobalCell>,
    /// Physical spill elevation in millimetres.
    pub sill: HeightMm,
    /// Exclusive receiving account beyond this spill.
    pub receiving: ReceivingAccount,
}

/// Zero-capacity equal-height join chains have already been coalesced.
/// Child IDs are canonically ordered; leaf IDs use minimum physical-cell anchors.
/// Logical Vec form is for checked bounded views only. Global storage uses a
/// fixed node row with child-table offset/count; root children stream separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasinNode {
    /// Stable identity of this coalesced node.
    pub id: BasinId,
    /// Containing basin, absent at a hierarchy root.
    pub parent: Option<BasinId>,
    /// Canonical physical minimum or canonical descendant anchor.
    pub anchor: GlobalCell,
    /// Lowest physical bed elevation represented by this node.
    pub floor: HeightMm,
    /// Canonical immediate child identities in this bounded logical view.
    pub children: Vec<BasinId>,
    /// Potential outward spill, independent of whether water currently reaches it.
    pub spill: Option<SpillConnection>,
}

/// Whole-domain annual balance after every internal transfer cancels.
/// All amounts are whole litres per representative 365-day year.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AnnualWaterBalance {
    /// Precipitation assigned to cells remaining dry land.
    pub land_precipitation: Litres,
    /// Effective dry-land loss, bounded by land precipitation.
    pub land_loss: Litres,
    /// Direct precipitation replacing runoff on strictly submerged cells.
    pub lake_precipitation: Litres,
    /// Open-water evaporation on strictly submerged cells.
    pub lake_evaporation: Litres,
    /// Explicit annual shallow-water adjustment for an unsupported raster band.
    pub marginal_evaporation: Litres,
    /// Annual discharge entering the connected modeled ocean.
    pub sea_outflow: Litres,
    /// Annual discharge leaving the modeled outer boundary.
    pub domain_outflow: Litres,
}

/// One actual connected positive-depth component in the annual representative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalLake {
    /// Stable identity shared by all fragments of this connected component.
    pub basin: BasinId,
    /// Whole-millimetre surface; membership is strictly physical bed < surface.
    pub surface: HeightMm,
    /// Lowest submerged physical bed elevation.
    pub deepest_bed: HeightMm,
    /// Number of cells with strictly positive depth.
    pub submerged_cells: u32,
    /// Potential physical spill, retained independently of supported overflow.
    pub outlet: Option<SpillConnection>,
    /// Supported whole litres per year, which may be positive when mean flow rounds to zero.
    pub annual_outflow: Litres,
    /// Annual outflow divided by 31,536,000 seconds, rounded down to whole L/s.
    pub mean_outflow: DischargeMilli,
}

/// Compact immutable contributing-owner identity, with no simulated reservoir state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnualCatchment {
    /// Stable contributing-owner identity used by reaches and crossings.
    pub catchment: CatchmentId,
    /// Actual physical routing terminal of this catchment.
    pub terminal: GlobalCell,
    /// Number of distinct physical source cells assigned to this owner.
    pub contributing_cells: u32,
    /// Immutable physical depression leaf, absent for direct open destinations.
    pub basin: Option<BasinId>,
    /// Actual positive-depth lake component containing the terminal, if wet.
    pub representative_lake: Option<BasinId>,
    /// Potential physical spill of a depression, absent for direct open destinations.
    pub potential_spill: Option<SpillConnection>,
    /// Physical downstream account; external IDs do not force recursive area loading.
    pub receiving: ReceivingAccount,
}

/// One annual discharge authority shared by every fragment of a course.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalReach {
    /// Canonical identity shared by all local fragments of this reach.
    pub id: ReachId,
    /// First actual modeled cell of the directed reach.
    pub from: GlobalCell,
    /// Last actual modeled cell; equal to `from` exactly for a point reach.
    pub to: GlobalCell,
    /// Exclusive account receiving this reach at its downstream end.
    pub receiving: ReceivingAccount,
    /// Canonical drainage ownership associated with this reach.
    pub catchment: CatchmentId,
    /// Number of distinct upstream contributing cells.
    pub drainage_cells: u32,
    /// Whole litres transported per representative year.
    pub annual_volume: Litres,
    /// Annual mean flow in whole litres per second.
    pub mean_discharge: DischargeMilli,
}

/// Both neighboring areas reference/copy this canonical directed record.
/// Annual transfer and mean discharge are copied from the canonical reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedCrossing {
    /// Canonical undirected endpoint identity of this shared adjacency.
    pub id: CrossingId,
    /// Actual upstream cell of this directed crossing.
    pub from: GlobalCell,
    /// Actual downstream D8 neighbor across the area boundary.
    pub to: GlobalCell,
    /// Canonical reach containing this crossing.
    pub reach: ReachId,
    /// Canonical contributing ownership at the crossing.
    pub catchment: CatchmentId,
    /// Number of distinct upstream contributing cells at the crossing.
    pub drainage_cells: u32,
    /// Whole litres transported across this adjacency per representative year.
    pub annual_volume: Litres,
    /// Annual mean crossing flow in whole litres per second.
    pub mean_discharge: DischargeMilli,
    /// Exclusive downstream account receiving the transported water.
    pub receiving: ReceivingAccount,
}

/// Saved render geometry derived exclusively from solved global courses.
/// Endpoint width at lake entry retains the incoming land width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelEdge {
    /// Upstream global centerline endpoint.
    pub from: GlobalCell,
    /// Downstream global centerline endpoint.
    pub to: GlobalCell,
    /// Full channel width at the upstream endpoint in decimetres.
    pub from_width_dm: u32,
    /// Full channel width at the downstream endpoint in decimetres.
    pub to_width_dm: u32,
    /// Authoritative annual mean discharge used for physical width.
    pub discharge: DischargeMilli,
}

/// Per-area lookup table, with full records stored once in global tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaHydrologyReferences {
    /// Exported area addressed by this lookup row.
    pub area: AreaCoord,
    /// Canonical identities of relevant annual lake components.
    pub lakes: Vec<BasinId>,
    /// Canonical identities of relevant reaches, including rendering context.
    pub reaches: Vec<ReachId>,
    /// Canonical identities of crossings relevant to the area.
    pub crossings: Vec<CrossingId>,
}

/// Exact relevant copies let an area explain saved water without neighbor reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AreaHydrologyContext {
    /// Model revision; annual semantics use revision two.
    pub model_revision: u32,
    /// Relevant global reach authorities.
    pub reaches: Vec<GlobalReach>,
    /// Relevant connected annual lake authorities.
    pub lakes: Vec<GlobalLake>,
    /// Relevant immutable catchment identities and destinations.
    pub catchments: Vec<AnnualCatchment>,
    /// Canonical cross-area transfers.
    pub crossings: Vec<SharedCrossing>,
}
impl Default for AreaHydrologyContext {
    fn default() -> Self {
        Self {
            model_revision: 2,
            reaches: Vec::new(),
            lakes: Vec::new(),
            catchments: Vec::new(),
            crossings: Vec::new(),
        }
    }
}

/// Persisted root metadata; large tables stream in separate checked sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydrologyMetadata {
    /// Supported annual model revision, currently two.
    pub model_revision: u32,
    /// Exact modeled rectangle and exported area dimensions.
    pub domain: HydrologyDomain,
    /// Number of coalesced physical basin nodes.
    pub basin_count: u64,
    /// Number of actual connected positive-depth components.
    pub lake_count: u64,
    /// Number of canonical directed reaches.
    pub reach_count: u64,
    /// Number of canonical directed area crossings.
    pub crossing_count: u64,
    /// Number of immutable contributing-owner summaries.
    pub catchment_count: u64,
    /// Whole-domain annual ledger after internal transfers cancel.
    pub budget: AnnualWaterBalance,
}

#[cfg(test)]
mod junction_id_tests {
    use super::*;
    #[test]
    fn actual_d8_steps_have_distinct_stable_ids_and_roundtrip_starts() {
        let from = GlobalCell { x: 17, y: 31 };
        let targets = [
            (16, 30),
            (17, 30),
            (18, 30),
            (16, 31),
            (18, 31),
            (16, 32),
            (17, 32),
            (18, 32),
        ];
        let base = JunctionId::at(from).0 * 8;
        for (direction, (x, y)) in (0_u64..).zip(targets) {
            let id = ReachId::from_step(from, GlobalCell { x, y });
            assert_eq!(id, Some(ReachId(base + direction)));
            assert_eq!(id.map(ReachId::start), Some(from));
        }
        assert_eq!(JunctionId::at(from).cell(), from);
        assert_eq!(ReachId::from_step(from, from), None);
        assert_eq!(ReachId::from_step(from, GlobalCell { x: 19, y: 31 }), None);
        let high = GlobalCell { x: 1, y: 1 << 29 };
        assert_eq!(
            ReachId::from_step(high, GlobalCell { x: 2, y: high.y }),
            None
        );
        let edge = GlobalCell {
            x: u32::MAX,
            y: u32::MAX,
        };
        assert_eq!(JunctionId::at(edge).cell(), edge);
    }
    #[test]
    fn point_ids_reserve_the_high_bit_without_truncating_coordinates() {
        let at = GlobalCell {
            x: u32::MAX,
            y: (1 << 28) - 1,
        };
        let point = ReachId::point(at).unwrap();
        assert!(point.is_point());
        assert_eq!(point.start(), at);
        let course = ReachId::from_step(
            at,
            GlobalCell {
                x: at.x - 1,
                y: at.y,
            },
        )
        .unwrap();
        assert!(!course.is_point());
        assert_ne!(course, point);
        assert_eq!(course.start(), at);
        let high = GlobalCell { x: 10, y: 1 << 28 };
        assert_eq!(
            ReachId::from_step(high, GlobalCell { x: 11, y: high.y }),
            None
        );
        assert_eq!(ReachId::point(high).unwrap().start(), high);
        assert_eq!(ReachId::point(GlobalCell { x: 0, y: 1 << 31 }), None);
    }
    #[test]
    fn shared_width_preserves_physical_floor_and_rejects_unrepresentable_values() {
        use super::channel_width_dm;
        use crate::DischargeMilli;
        for (flow, width) in [
            (0, 0),
            (40, 8),
            (999, 39),
            (1000, 40),
            (25_000, 200),
            (33_249_619_483, 230_649),
        ] {
            assert_eq!(channel_width_dm(DischargeMilli::new(flow)), Some(width));
        }
        assert_eq!(channel_width_dm(DischargeMilli::new(u64::MAX)), None);
    }
}
