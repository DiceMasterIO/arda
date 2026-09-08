//! Compose an exported area from immutable prepared cells and completed shared authority.
#![deny(missing_docs)]

use super::extraction::{self, ExtractionBudget, OwnedChannelCell};
use super::final_index::FinalIndex;
use super::fine_flow::FlowStore;
use super::local_rivers;
use super::prepared_codec::PreparedTile;
use super::routing::{CellIndex, RoutingStore};
use super::types::PreparedTerrain;
use crate::area::shared_compose::{self, SolvedCell};
use arda_core::hydrology::{
    self, AnnualCatchment, AreaHydrologyContext, BasinId, CatchmentId, ChannelEdge, CrossingId,
    GlobalLake, GlobalReach, ReceivingAccount, SharedCrossing,
};
use arda_core::{AreaCells, AreaObjects, DischargeMilli, GlobalCell};
use std::collections::BTreeMap;
use std::convert::Infallible;

const CELLS: usize = 512 * 512;
const YEAR: u64 = 31_536_000;

/// Own composition admission; final encoding and borrowed shared stores reserve separately.
#[derive(Debug, Clone, Copy)]
pub struct AreaLimits {
    /// Own prepared copies, solved cells, local objects and context/index overlap.
    pub ram_bytes: u64,
    /// Actual source reads, extraction work and context/geometry visits.
    pub operations: u64,
    /// Maximum total copied context rows, no greater than the saved decoder limit.
    pub context_records: u32,
    /// Maximum physical D8 edges, no greater than the saved renderer/decoder limit.
    pub channels: u32,
}

/// Typed original read failure, invalid completed authority or explicit admission failure.
#[derive(Debug, thiserror::Error)]
pub enum AreaError<R, F> {
    /// Physical routing source failed.
    #[error("area output terrain read failed")]
    Routing(#[source] R),
    /// Completed physical flow/metrics source failed.
    #[error("area output flow read failed")]
    Flow(#[source] F),
    /// Saved local channel authority failed.
    #[error("area output channel extraction failed")]
    Extract(#[source] extraction::ExtractError<R, F>),
    /// Physical terrain/lake cell composition failed.
    #[error(transparent)]
    Compose(#[from] shared_compose::ComposeError),
    /// Disjoint local course assembly failed.
    #[error("area output local river assembly failed")]
    Rivers(#[source] local_rivers::LocalRiverError<Infallible>),
    /// Prepared, indexed and physical authorities disagree.
    #[error("invalid shared area output: {0}")]
    Invalid(&'static str),
    /// Checked allocation, RAM, record, width or work bound failed.
    #[error("shared area output resource limit")]
    Limit,
}

/// Conservative own payload including local-river construction and context map conversion.
#[must_use]
pub fn required_ram(channel_cells: u64, context_rows: u64, channels: u64) -> Option<u64> {
    (64_u64 * 1024 * 1024)
        .checked_add(channel_cells.checked_mul(640)?)?
        .checked_add(context_rows.checked_mul(512)?)?
        .checked_add(channels.checked_mul(128)?)
}

struct Context<'a> {
    index: &'a FinalIndex,
    global_lakes: &'a [GlobalLake],
    limits: AreaLimits,
    work: ExtractionBudget,
    owned: u64,
    reaches: Vec<GlobalReach>,
    lakes: BTreeMap<BasinId, GlobalLake>,
    catchments: BTreeMap<CatchmentId, AnnualCatchment>,
    crossings: BTreeMap<CrossingId, SharedCrossing>,
    channels: BTreeMap<(GlobalCell, GlobalCell), ChannelEdge>,
}
impl Context<'_> {
    fn charge<R, F>(&mut self, n: u64) -> Result<(), AreaError<R, F>> {
        self.work.remaining = self.work.remaining.checked_sub(n).ok_or(AreaError::Limit)?;
        Ok(())
    }
    fn admit<R, F>(
        &self,
        extra_rows: u64,
        extra_owned: u64,
        extra_channels: u64,
    ) -> Result<(), AreaError<R, F>> {
        let rows = u64::try_from(
            self.reaches.len() + self.lakes.len() + self.catchments.len() + self.crossings.len(),
        )
        .map_err(|_| AreaError::Limit)?
        .checked_add(extra_rows)
        .ok_or(AreaError::Limit)?;
        let channels = u64::try_from(self.channels.len())
            .map_err(|_| AreaError::Limit)?
            .checked_add(extra_channels)
            .ok_or(AreaError::Limit)?;
        let owned = self
            .owned
            .checked_add(extra_owned)
            .ok_or(AreaError::Limit)?;
        if rows > u64::from(self.limits.context_records)
            || channels > u64::from(self.limits.channels)
            || owned > CELLS as u64
            || required_ram(owned, rows, channels).is_none_or(|b| b > self.limits.ram_bytes)
        {
            return Err(AreaError::Limit);
        }
        Ok(())
    }
    fn lake<R, F>(&mut self, id: BasinId) -> Result<(), AreaError<R, F>> {
        self.charge(1)?;
        if self.lakes.contains_key(&id) {
            return Ok(());
        }
        let i = self
            .global_lakes
            .binary_search_by_key(&id, |l| l.basin)
            .map_err(|_| AreaError::Invalid("missing connected lake"))?;
        self.admit(1, 0, 0)?;
        self.lakes.insert(id, self.global_lakes[i].clone());
        Ok(())
    }
    fn catchment<R, F>(&mut self, id: CatchmentId) -> Result<(), AreaError<R, F>> {
        self.charge(1)?;
        if self.catchments.contains_key(&id) {
            return Ok(());
        }
        let i = self
            .index
            .catchments
            .binary_search_by_key(&id, |c| c.catchment)
            .map_err(|_| AreaError::Invalid("missing catchment"))?;
        let row = self.index.catchments[i].clone();
        if let Some(lake) = row.representative_lake {
            self.lake(lake)?;
        }
        self.admit(1, 0, 0)?;
        self.catchments.insert(id, row);
        Ok(())
    }
    fn reach<R, F>(&mut self, ordinal: usize) -> Result<(), AreaError<R, F>> {
        self.charge(1)?;
        let row = self
            .index
            .reaches
            .get(ordinal)
            .cloned()
            .ok_or(AreaError::Invalid("area reach ordinal"))?;
        if self.reaches.last().is_some_and(|r| r.id >= row.id) {
            return Err(AreaError::Invalid("area reach order"));
        }
        self.catchment(row.catchment)?;
        if let ReceivingAccount::Lake(id) = row.receiving {
            if self
                .global_lakes
                .binary_search_by_key(&id, |l| l.basin)
                .is_ok()
            {
                self.lake(id)?;
            } else {
                self.catchment(CatchmentId(id.0))?;
            }
        }
        if row.from.x / 512 != row.to.x / 512 || row.from.y / 512 != row.to.y / 512 {
            let id = CrossingId {
                low: row.from.min(row.to),
                high: row.from.max(row.to),
            };
            let at = self
                .index
                .crossings
                .binary_search_by_key(&id, |c| c.id)
                .map_err(|_| AreaError::Invalid("missing shared crossing"))?;
            let crossing = &self.index.crossings[at];
            if crossing.reach != row.id {
                return Err(AreaError::Invalid("crossing reach identity"));
            }
            self.admit(1, 0, 0)?;
            if self.crossings.insert(id, crossing.clone()).is_some() {
                return Err(AreaError::Invalid("duplicate copied crossing"));
            }
        }
        if row.from != row.to && row.mean_discharge.raw() >= 40 {
            let width = hydrology::channel_width_dm(row.mean_discharge)
                .filter(|w| *w > 0)
                .ok_or(AreaError::Limit)?;
            self.admit(0, 0, 1)?;
            if self
                .channels
                .insert(
                    (row.from, row.to),
                    ChannelEdge {
                        from: row.from,
                        to: row.to,
                        from_width_dm: width,
                        to_width_dm: width,
                        discharge: row.mean_discharge,
                    },
                )
                .is_some()
            {
                return Err(AreaError::Invalid("duplicate physical geometry"));
            }
        }
        self.admit(1, 0, 0)?;
        self.reaches.try_reserve(1).map_err(|_| AreaError::Limit)?;
        self.reaches.push(row);
        Ok(())
    }
}

fn reserve<T, R, F>(count: usize) -> Result<Vec<T>, AreaError<R, F>> {
    let mut out = Vec::new();
    out.try_reserve_exact(count).map_err(|_| AreaError::Limit)?;
    Ok(out)
}
fn values<K, V, R, F>(map: BTreeMap<K, V>) -> Result<Vec<V>, AreaError<R, F>> {
    let mut out = reserve(map.len())?;
    out.extend(map.into_values());
    Ok(out)
}

/// Build one final area without local routing, source recomputation or neighbor area reads.
/// The caller keeps private prepared/routing/flow readers and the shared index alive.
/// Final encoding validates the saved record limits before any completion manifest exists.
#[allow(clippy::too_many_arguments)]
pub fn compose<R: RoutingStore, F: FlowStore>(
    tile: &PreparedTile,
    routing: &mut R,
    flow: &mut F,
    index: &FinalIndex,
    lakes: &[GlobalLake],
    limits: AreaLimits,
) -> Result<(AreaCells, AreaObjects), AreaError<R::Error, F::Error>> {
    let extent = routing.extent();
    let last =
        CellIndex::new(extent.cells() - 1, extent).ok_or(AreaError::Invalid("empty extent"))?;
    let (mx, my) = extent.coordinates(last);
    let valid = tile.valid();
    if u64::try_from(lakes.len()).map_err(|_| AreaError::Limit)? > limits.operations {
        return Err(AreaError::Limit);
    }
    if flow.extent() != extent
        || index.domain.width_cells != mx + 1
        || index.domain.height_cells != my + 1
        || index.domain.exported_areas_wide > (mx + 1) / 512
        || index.domain.exported_areas_high > (my + 1) / 512
        || valid.width != 512
        || valid.height != 512
        || tile.cells().len() != CELLS
        || limits.context_records > 1_048_576
        || limits.channels > 262144
        || lakes.windows(2).any(|l| l[0].basin >= l[1].basin)
    {
        return Err(AreaError::Invalid(
            "shape, schema limits or connected lake order",
        ));
    }
    let relevant = index
        .area(tile.area())
        .ok_or(AreaError::Invalid("area is not exported"))?;
    let mut context = Context {
        index,
        global_lakes: lakes,
        limits,
        work: ExtractionBudget {
            remaining: limits.operations,
        },
        owned: 0,
        reaches: Vec::new(),
        lakes: BTreeMap::new(),
        catchments: BTreeMap::new(),
        crossings: BTreeMap::new(),
        channels: BTreeMap::new(),
    };
    context.admit(0, 0, 0)?;
    context.charge(u64::try_from(lakes.len()).map_err(|_| AreaError::Limit)?)?;
    let ax = u32::try_from(tile.area().x).map_err(|_| AreaError::Invalid("negative area"))? * 512;
    let ay = u32::try_from(tile.area().y).map_err(|_| AreaError::Invalid("negative area"))? * 512;
    let mut prepared = PreparedTerrain {
        area: tile.area(),
        valid,
        heights: reserve(CELLS)?,
        annual_rain: reserve(CELLS)?,
        temperature_base_centi: reserve(CELLS)?,
    };
    let mut solved = reserve(CELLS)?;
    let mut owned: Vec<OwnedChannelCell> = Vec::new();
    for (i, cell) in tile.cells().iter().enumerate() {
        context.charge(2)?;
        let raw = u32::try_from(i).map_err(|_| AreaError::Limit)?;
        let x = ax + raw % 512;
        let y = ay + raw / 512;
        let at = CellIndex::new(y * index.domain.width_cells + x, extent)
            .ok_or(AreaError::Invalid("prepared coordinate"))?;
        let terrain = routing.read(at).map_err(AreaError::Routing)?;
        let row = flow.read(at).map_err(AreaError::Flow)?;
        if terrain.height() != cell.height.raw() {
            return Err(AreaError::Invalid("prepared physical height disagreement"));
        }
        prepared.heights.push(cell.height);
        prepared.annual_rain.push(cell.annual_rain);
        prepared
            .temperature_base_centi
            .push(cell.temperature_base_centi);
        let mut water = SolvedCell {
            marine: terrain.is_marine(),
            lake: row.lake,
            ..SolvedCell::default()
        };
        if let Some(lake) = row.lake {
            context.lake(lake)?;
            if context
                .lakes
                .get(&lake)
                .is_none_or(|global| global.surface.raw() != row.surface_mm)
            {
                return Err(AreaError::Invalid("flow/global lake surface disagreement"));
            }
        }
        if !water.marine && water.lake.is_none() {
            water.drainage_cells = row.metrics.drainage_cells;
            water.discharge = DischargeMilli::new(row.metrics.scalar_annual / YEAR);
            water.order = row.metrics.order;
            if let Some(reference) = row.metrics.hand_at {
                if reference == at {
                    water.hand_mm = 0;
                } else {
                    context.charge(2)?;
                    let reference_terrain = routing.read(reference).map_err(AreaError::Routing)?;
                    let height = reference_terrain.height();
                    let channel = flow.read(reference).map_err(AreaError::Flow)?;
                    let q = channel.metrics.scalar_annual / YEAR;
                    if q < 40 || channel.lake.is_some() || reference_terrain.is_marine() {
                        return Err(AreaError::Invalid("HAND reference is not a dry channel"));
                    }
                    water.hand_mm =
                        u32::try_from((i64::from(terrain.height()) - i64::from(height)).max(0))
                            .map_err(|_| AreaError::Limit)?;
                    water.hand_discharge = DischargeMilli::new(q);
                }
            }
        }
        solved.push(water);
        let batch =
            extraction::cell(routing, flow, at, &mut context.work).map_err(AreaError::Extract)?;
        if let Some(channel) = batch.owned {
            context.admit(0, 1, 0)?;
            owned.try_reserve(1).map_err(|_| AreaError::Limit)?;
            owned.push(channel);
            context.owned += 1;
        }
    }
    for &ordinal in relevant {
        context.reach(ordinal)?;
    }
    let row_count = u64::try_from(
        context.reaches.len()
            + context.lakes.len()
            + context.catchments.len()
            + context.crossings.len()
            + context.channels.len(),
    )
    .map_err(|_| AreaError::Limit)?;
    context.charge(
        row_count
            .checked_add(CELLS as u64)
            .ok_or(AreaError::Limit)?,
    )?;
    let local_count = u32::try_from(owned.len()).map_err(|_| AreaError::Limit)?;
    let rivers = local_rivers::compose(
        tile.area(),
        owned.into_iter().map(Ok::<_, Infallible>),
        local_count,
        local_rivers::required_ram(local_count).ok_or(AreaError::Limit)?,
    )
    .map_err(AreaError::Rivers)?;
    let copied_lakes: Vec<GlobalLake> = values(context.lakes)?;
    let (cells, local_lakes) = shared_compose::compose_shared(
        &prepared,
        &solved,
        &copied_lakes,
        shared_compose::required_ram(copied_lakes.len()).ok_or(AreaError::Limit)?,
    )?;
    Ok((
        cells,
        AreaObjects {
            channel_edges: values(context.channels)?,
            rivers,
            lakes: local_lakes,
            global: AreaHydrologyContext {
                model_revision: 2,
                reaches: context.reaches,
                lakes: copied_lakes,
                catchments: values(context.catchments)?,
                crossings: values(context.crossings)?,
            },
        },
    ))
}
