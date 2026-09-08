//! Streaming annual source and support-band aggregation over immutable physical cells.
#![deny(missing_docs)]

use super::annual::{AnnualBand, AnnualNode, MAX_ANNUAL};
use super::routing::Extent;
use arda_core::hydrology::{BasinId, Litres};
use arda_core::{GlobalCell, HeightMm, RainfallMm};
use std::collections::BTreeMap;

/// Physical terminal established by the shared receiver pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    /// Immutable closed catchment leaf.
    Basin(BasinId),
    /// Nonmarine land draining directly to connected ocean.
    Sea,
    /// Nonmarine land draining through the actual modeled rim.
    DomainExport,
    /// Connected ocean; its rainfall is outside the terrestrial water ledger.
    Marine,
}

/// One prepared cell, delivered exactly once in global row-major order.
#[derive(Debug, Clone)]
pub struct AnnualCell {
    /// Absolute modeled coordinate.
    pub at: GlobalCell,
    /// Immutable physical bed, without routing epsilon.
    pub bed: HeightMm,
    /// Prepared canonical annual precipitation.
    pub rain: RainfallMm,
    /// Twelve climatological evaporation depths, already sampled at physical temperature.
    pub evaporation_um: [u32; 12],
    /// Actual receiver ownership and marine classification.
    pub terminal: Terminal,
}

/// Exact annual effective-source terms for one 100 m square cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellBudget {
    /// Precipitation in whole litres/year.
    pub precipitation: Litres,
    /// Open-water evaporation in whole litres/year.
    pub evaporation: Litres,
    /// Effective baseline land loss min(P/2,E).
    pub land_loss: Litres,
    /// Effective runoff P-A.
    pub runoff: Litres,
}

/// Sums monthly evaporation; evaluating evaporation at annual mean temperature is not equivalent.
#[must_use]
pub fn cell_budget(rain: RainfallMm, evaporation_um: &[u32; 12]) -> CellBudget {
    let p = u128::from(rain.raw()) * 10_000;
    let e = evaporation_um.iter().map(|&x| u128::from(x)).sum::<u128>() * 10;
    let a = (p / 2).min(e);
    CellBudget {
        precipitation: Litres(p),
        evaporation: Litres(e),
        land_loss: Litres(a),
        runoff: Litres(p - a),
    }
}

/// Nonbasin terrestrial sources pass directly to their actual exterior destination.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DirectSource {
    /// Number of nonmarine contributing cells.
    pub cells: u32,
    /// Annual precipitation.
    pub precipitation: Litres,
    /// Annual effective land loss.
    pub land_loss: Litres,
    /// Annual export after effective land loss.
    pub runoff: Litres,
}

/// Admitted streaming working set, including owned nodes, map entries and output overlap.
#[derive(Debug, Clone, Copy)]
pub struct AggregationLimits {
    /// Own payload reservation; unrelated readers and routing pages are reserved separately.
    pub ram_bytes: u64,
    /// Maximum distinct (node, exact bed, immutable owner) bands.
    pub bands: u64,
    /// Node visits, cell visits, ancestry steps and final band rows.
    pub operations: u64,
}

/// Invalid authority or an admitted resource bound aborts the unpublished result.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AggregationError {
    /// Invalid ordering, ownership or hierarchy shape.
    #[error("invalid annual aggregation: {0}")]
    Invalid(&'static str),
    /// Whole-litre arithmetic or a source bound failed.
    #[error("annual aggregation arithmetic overflow")]
    Overflow,
    /// RAM, row count, allocation or counted work exceeded admission.
    #[error("annual aggregation resource limit: {0}")]
    Limit(&'static str),
}
type Result<T> = std::result::Result<T, AggregationError>;

/// Bound includes tree-node overhead and simultaneous map-to-vector conversion.
#[must_use]
pub fn required_ram(nodes: u64, bands: u64) -> Option<u64> {
    nodes
        .checked_mul(512)?
        .checked_add(bands.checked_mul(512)?)?
        .checked_add(8192)
}
fn add(a: u128, b: u128) -> Result<u128> {
    a.checked_add(b)
        .filter(|&n| n <= MAX_ANNUAL)
        .ok_or(AggregationError::Overflow)
}
fn source_add(source: &mut DirectSource, b: CellBudget) -> Result<()> {
    source.cells = source
        .cells
        .checked_add(1)
        .ok_or(AggregationError::Overflow)?;
    source.precipitation.0 = add(source.precipitation.0, b.precipitation.0)?;
    source.land_loss.0 = add(source.land_loss.0, b.land_loss.0)?;
    source.runoff.0 = add(source.runoff.0, b.runoff.0)?;
    Ok(())
}

/// Complete source and band input; child and join arrays remain with the hierarchy adapter.
#[derive(Debug)]
pub struct Aggregation {
    /// Canonical nodes with source totals and disjoint band spans installed.
    pub nodes: Vec<AnnualNode>,
    /// Ordered by node, physical bed and immutable owner.
    pub bands: Vec<AnnualBand>,
    /// Full catchment cell counts, parallel to nodes; internal nodes remain zero.
    pub catchment_cells: Vec<u32>,
    /// Direct terrestrial ocean export.
    pub sea: DirectSource,
    /// Direct terrestrial domain export.
    pub domain: DirectSource,
    /// Connected marine cells omitted from terrestrial sources.
    pub marine_cells: u32,
    /// Counted node, cell, ancestor and final-band operations.
    pub operations: u64,
}

/// Owns only admitted summaries and grouped bands; it never stores an annual fine-cell array.
pub struct Aggregator {
    extent: Extent,
    nodes: Vec<AnnualNode>,
    parents: Vec<Option<usize>>,
    counts: Vec<u32>,
    bands: BTreeMap<(usize, i32, BasinId), AnnualBand>,
    sea: DirectSource,
    domain: DirectSource,
    marine: u32,
    next: u32,
    operations: u64,
    limits: AggregationLimits,
    total_p: u128,
    total_e: u128,
    node_slots: u64,
    failed: bool,
}

impl Aggregator {
    /// Admits nodes before allocating indices; all source totals and band spans must be empty.
    pub fn new(extent: Extent, nodes: Vec<AnnualNode>, limits: AggregationLimits) -> Result<Self> {
        let count = u64::try_from(nodes.len()).map_err(|_| AggregationError::Overflow)?;
        let node_slots = u64::try_from(nodes.capacity()).map_err(|_| AggregationError::Overflow)?;
        let operations = count.checked_mul(3).ok_or(AggregationError::Overflow)?;
        if operations > limits.operations {
            return Err(AggregationError::Limit("operations"));
        }
        if required_ram(node_slots, 0).is_none_or(|bytes| bytes > limits.ram_bytes) {
            return Err(AggregationError::Limit("RAM"));
        }
        let mut parents = Vec::new();
        let mut counts = Vec::<u32>::new();
        parents
            .try_reserve_exact(nodes.len())
            .map_err(|_| AggregationError::Limit("allocation"))?;
        counts
            .try_reserve_exact(nodes.len())
            .map_err(|_| AggregationError::Limit("allocation"))?;
        for (i, n) in nodes.iter().enumerate() {
            if i > 0 && nodes[i - 1].id >= n.id {
                return Err(AggregationError::Invalid("node order"));
            }
            if n.floor > n.birth
                || n.spill.is_none_or(|s| s.sill <= n.birth)
                || n.local_runoff.0 != 0
                || n.local_precipitation.0 != 0
                || n.local_land_loss.0 != 0
                || !n.bands.is_empty()
                || n.children.start > n.children.end
                || n.bands.start > n.bands.end
                || (n.children.is_empty() && n.floor != n.birth)
            {
                return Err(AggregationError::Invalid("node shape or populated input"));
            }
            let parent = n
                .parent
                .map(|id| {
                    nodes
                        .binary_search_by_key(&id, |n| n.id)
                        .map_err(|_| AggregationError::Invalid("missing parent"))
                })
                .transpose()?;
            if let Some(p) = parent {
                if p == i
                    || nodes[p].birth <= n.birth
                    || nodes[p].floor > n.floor
                    || n.spill.is_none_or(|s| s.sill != nodes[p].birth)
                {
                    return Err(AggregationError::Invalid("parent event"));
                }
            }
            parents.push(parent);
            counts.push(0);
        }
        for &parent in &parents {
            if let Some(parent) = parent {
                counts[parent] = counts[parent]
                    .checked_add(1)
                    .ok_or(AggregationError::Overflow)?;
            }
        }
        for (i, n) in nodes.iter().enumerate() {
            if usize::try_from(counts[i]).map_err(|_| AggregationError::Overflow)?
                != n.children.len()
                || counts[i] == 1
            {
                return Err(AggregationError::Invalid("child count"));
            }
            counts[i] = 0;
        }
        Ok(Self {
            extent,
            nodes,
            parents,
            counts,
            bands: BTreeMap::new(),
            sea: DirectSource::default(),
            domain: DirectSource::default(),
            marine: 0,
            next: 0,
            operations,
            limits,
            total_p: 0,
            total_e: 0,
            node_slots,
            failed: false,
        })
    }

    fn tick(&mut self) -> Result<()> {
        self.operations = self
            .operations
            .checked_add(1)
            .ok_or(AggregationError::Overflow)?;
        if self.operations > self.limits.operations {
            return Err(AggregationError::Limit("operations"));
        }
        Ok(())
    }

    /// Adds one cell. On any error, discard this accumulator; no partial result is publishable.
    pub fn push(&mut self, cell: &AnnualCell) -> Result<()> {
        if self.failed {
            return Err(AggregationError::Invalid("failed accumulator"));
        }
        let result = self.push_inner(cell);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn push_inner(&mut self, cell: &AnnualCell) -> Result<()> {
        self.tick()?;
        let index = super::routing::CellIndex::new(self.next, self.extent)
            .ok_or(AggregationError::Invalid("too many cells"))?;
        if self.extent.coordinates(index) != (cell.at.x, cell.at.y) {
            return Err(AggregationError::Invalid("cell order or coverage"));
        }
        self.next += 1;
        if cell.terminal == Terminal::Marine {
            if cell.bed.raw() > 0 {
                return Err(AggregationError::Invalid("positive marine bed"));
            }
            self.marine = self
                .marine
                .checked_add(1)
                .ok_or(AggregationError::Overflow)?;
            return Ok(());
        }
        let b = cell_budget(cell.rain, &cell.evaporation_um);
        self.total_p = add(self.total_p, b.precipitation.0)?;
        self.total_e = add(self.total_e, b.evaporation.0)?;
        let owner = match cell.terminal {
            Terminal::Sea => return source_add(&mut self.sea, b),
            Terminal::DomainExport => return source_add(&mut self.domain, b),
            Terminal::Basin(id) => id,
            Terminal::Marine => return Err(AggregationError::Invalid("marine source")),
        };
        let leaf = self
            .nodes
            .binary_search_by_key(&owner, |n| n.id)
            .map_err(|_| AggregationError::Invalid("unknown owner"))?;
        let n = &mut self.nodes[leaf];
        if !n.children.is_empty() || cell.bed < n.floor {
            return Err(AggregationError::Invalid(
                "nonleaf owner or bed below floor",
            ));
        }
        n.local_precipitation.0 = add(n.local_precipitation.0, b.precipitation.0)?;
        n.local_land_loss.0 = add(n.local_land_loss.0, b.land_loss.0)?;
        n.local_runoff.0 = add(n.local_runoff.0, b.runoff.0)?;
        self.counts[leaf] = self.counts[leaf]
            .checked_add(1)
            .ok_or(AggregationError::Overflow)?;
        let mut node = leaf;
        let mut steps = 0;
        loop {
            self.tick()?;
            steps += 1;
            if steps > self.nodes.len() {
                return Err(AggregationError::Invalid("parent cycle"));
            }
            let n = &self.nodes[node];
            let spill = n
                .spill
                .ok_or(AggregationError::Invalid("missing finite spill"))?;
            if cell.bed < n.birth {
                return Err(AggregationError::Invalid("gap in active range"));
            }
            if cell.bed < spill.sill {
                break;
            }
            match self.parents[node] {
                Some(p) => node = p,
                None => return Ok(()), // Source above reachable storage still contributes runoff.
            }
        }
        let key = (node, cell.bed.raw(), owner);
        if !self.bands.contains_key(&key) {
            let rows = u64::try_from(self.bands.len()).map_err(|_| AggregationError::Overflow)? + 1;
            if rows > self.limits.bands
                || required_ram(self.node_slots, rows).is_none_or(|n| n > self.limits.ram_bytes)
            {
                return Err(AggregationError::Limit("bands or RAM"));
            }
            self.bands.insert(
                key,
                AnnualBand {
                    bed: cell.bed,
                    owner,
                    cells: 0,
                    precipitation: Litres(0),
                    effective_land_loss: Litres(0),
                    open_water_evaporation: Litres(0),
                },
            );
        }
        let band = self
            .bands
            .get_mut(&key)
            .ok_or(AggregationError::Invalid("band insertion"))?;
        band.cells = band
            .cells
            .checked_add(1)
            .ok_or(AggregationError::Overflow)?;
        band.precipitation.0 = add(band.precipitation.0, b.precipitation.0)?;
        band.effective_land_loss.0 = add(band.effective_land_loss.0, b.land_loss.0)?;
        band.open_water_evaporation.0 = add(band.open_water_evaporation.0, b.evaporation.0)?;
        Ok(())
    }

    /// Verifies complete coverage and installs canonical, nonoverlapping band spans.
    pub fn finish(mut self) -> Result<Aggregation> {
        if self.failed {
            return Err(AggregationError::Invalid("failed accumulator"));
        }
        if self.next != self.extent.cells() {
            return Err(AggregationError::Invalid("incomplete domain"));
        }
        let rows = u64::try_from(self.bands.len()).map_err(|_| AggregationError::Overflow)?;
        self.operations = self
            .operations
            .checked_add(rows)
            .and_then(|n| n.checked_add(u64::try_from(self.nodes.len()).ok()?))
            .ok_or(AggregationError::Overflow)?;
        if self.operations > self.limits.operations {
            return Err(AggregationError::Limit("operations"));
        }
        let mut bands = Vec::new();
        bands
            .try_reserve_exact(self.bands.len())
            .map_err(|_| AggregationError::Limit("allocation"))?;
        let mut rows = self.bands.into_iter().peekable();
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let start = bands.len();
            while rows.peek().is_some_and(|((owner, _, _), _)| *owner == i) {
                if let Some((_, band)) = rows.next() {
                    bands.push(band);
                }
            }
            node.bands = start..bands.len();
        }
        if rows.next().is_some() {
            return Err(AggregationError::Invalid("unassigned bands"));
        }
        Ok(Aggregation {
            nodes: self.nodes,
            bands,
            catchment_cells: self.counts,
            sea: self.sea,
            domain: self.domain,
            marine_cells: self.marine,
            operations: self.operations,
        })
    }
}

#[cfg(test)]
#[path = "annual_aggregation_tests.rs"]
mod tests;
