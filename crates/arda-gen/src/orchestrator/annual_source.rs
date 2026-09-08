//! Canonical prepared-to-annual sources with bounded per-tile-row climate windows.
#![deny(missing_docs)]
use super::prepared_files::{PreparedError, PreparedReader};
use crate::area::temperature::temperature_from_base;
use crate::continent::{climate::ContinentClimate, ContinentGrid};
use crate::hydrology::annual::LeafNet;
use crate::hydrology::annual_aggregation::{cell_budget, AnnualCell, Terminal};
use crate::hydrology::budget::BudgetError;
use crate::hydrology::fine_flow::CellSource;
use crate::hydrology::forcing::YearForcing;
use crate::hydrology::forcing_sample::ForcingWindow;
use crate::hydrology::prepared_domain::PreparedDomain;
use crate::hydrology::routing::{CellIndex, Extent, OutletKind, Receiver, RoutingStore};
use arda_core::hydrology::{BasinId, HydrologyDomain, Litres};
use arda_core::{AreaCoord, GenerateConfig, GlobalCell, HeightMm, RainfallMm};

const WINDOW_NODES: u64 = 55 * 55;
const WINDOW_WORK: u64 = WINDOW_NODES * 12;
const SAMPLE_WORK: u64 = 24; // original interpolated year plus physical-temperature E

/// Independent source cache/work reservation; reader and routing pages are separate.
#[derive(Debug, Clone, Copy)]
pub struct SourceLimits {
    /// Own ForcingWindow/cache/reference payload only.
    pub ram_bytes: u64,
    /// Cell/query/month-profile work events, admitted before reading.
    pub operations: u64,
}
/// Actual source work; values do not depend on cache residency in other stages.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SourceWork {
    /// Counted source/query/month-profile work events.
    pub operations: u64,
    /// Successfully produced modeled cells.
    pub cells: u64,
    /// Coarse area windows created; at most domain.count() per full pass.
    pub windows: u64,
    /// Nonmarine physical-temperature monthly samples.
    pub samples: u64,
}
/// Prepared/forcing/admission failure, independent of a routing backend's error type.
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    /// Exact prepared file, checksum or crop failure.
    #[error(transparent)]
    Prepared(#[from] PreparedError),
    /// Exact selected integer forcing failure.
    #[error(transparent)]
    Forcing(#[from] BudgetError),
    /// Input authority or call order is inconsistent.
    #[error("invalid annual source: {0}")]
    Invalid(&'static str),
    /// Cache or logical work was not admitted.
    #[error("annual source exceeded {0}")]
    Limit(&'static str),
}
/// Routing reads preserve their original backend failure without shortening the source.
#[derive(Debug, thiserror::Error)]
pub enum CellSourceError<E> {
    /// Source/forcing/prepared error.
    #[error(transparent)]
    Source(#[from] SourceError),
    /// Original immutable routing read error.
    #[error("annual source routing read failed")]
    Routing(#[source] E),
}
/// At most one55²-node window per prepared column, plus metadata and temporary sampling state.
#[must_use]
pub fn required_ram(domain: PreparedDomain) -> Option<u64> {
    let year = u64::try_from(std::mem::size_of::<YearForcing>()).ok()?;
    WINDOW_NODES
        .checked_mul(year)?
        .checked_add(512)?
        .checked_mul(u64::from(domain.columns()))?
        .checked_add(16_384)
}
/// Complete pass upper bound, including one optional <=N final-leaf validation.
/// Backend query events are distinct from the readers' admitted actual byte I/O.
#[must_use]
pub fn required_operations(domain: PreparedDomain) -> Option<u64> {
    let cells = u64::from(domain.width()).checked_mul(u64::from(domain.height()))?;
    cells
        .checked_mul(SAMPLE_WORK + 5 + 31)?
        .checked_add(u64::from(domain.count()).checked_mul(WINDOW_WORK + 1)?)
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Unset,
    Annual,
    Fine,
}

/// Owns only a prepared-reader borrow and bounded climate cache. Routing is borrowed
/// for each call, allowing fine_flow::run_with_source to lend its existing backend.
/// Start a fresh source for each complete annual/fine pass; do not mix call modes.
pub struct AnnualSource<'a> {
    prepared: &'a mut PreparedReader,
    grid: &'a ContinentGrid,
    climate: &'a ContinentClimate,
    config: GenerateConfig,
    domain: PreparedDomain,
    extent: Extent,
    forcing_domain: HydrologyDomain,
    windows: Vec<Option<ForcingWindow>>,
    tile_row: u32,
    next: u32,
    mode: Mode,
    failed: bool,
    limits: SourceLimits,
    work: SourceWork,
}
impl<'a> AnnualSource<'a> {
    /// Admits a complete canonical scan and validates the exact request-derived domain.
    /// No routing borrow or fine/monthly array is retained.
    pub fn new(
        prepared: &'a mut PreparedReader,
        grid: &'a ContinentGrid,
        climate: &'a ContinentClimate,
        config: GenerateConfig,
        limits: SourceLimits,
    ) -> Result<Self, SourceError> {
        let domain = PreparedDomain::for_config(config)
            .map_err(|_| SourceError::Invalid("request domain"))?;
        let size = config.size_km();
        if domain != prepared.domain()
            || u32::try_from(grid.width()).ok() != Some(size.width)
            || u32::try_from(grid.height()).ok() != Some(size.height)
        {
            return Err(SourceError::Invalid("prepared/climate request geometry"));
        }
        if required_ram(domain).is_none_or(|n| n > limits.ram_bytes)
            || required_operations(domain).is_none_or(|n| n > limits.operations)
        {
            return Err(SourceError::Limit("cache or operations admission"));
        }
        let extent = Extent::new(domain.width(), domain.height())
            .ok_or(SourceError::Invalid("routing extent"))?;
        let forcing_domain = HydrologyDomain {
            width_cells: domain.width(),
            height_cells: domain.height(),
            exported_areas_wide: u32::try_from(config.areas_wide())
                .map_err(|_| SourceError::Invalid("export columns"))?,
            exported_areas_high: u32::try_from(config.areas_high())
                .map_err(|_| SourceError::Invalid("export rows"))?,
        };
        let mut windows = Vec::new();
        let columns =
            usize::try_from(domain.columns()).map_err(|_| SourceError::Limit("columns"))?;
        windows
            .try_reserve_exact(columns)
            .map_err(|_| SourceError::Limit("window allocation"))?;
        windows.resize_with(columns, || None);
        Ok(Self {
            prepared,
            grid,
            climate,
            config,
            domain,
            extent,
            forcing_domain,
            windows,
            tile_row: 0,
            next: 0,
            mode: Mode::Unset,
            failed: false,
            limits,
            work: SourceWork::default(),
        })
    }
    fn charge(&mut self, amount: u64) -> Result<(), SourceError> {
        self.work.operations = self
            .work
            .operations
            .checked_add(amount)
            .ok_or(SourceError::Limit("operations"))?;
        if self.work.operations > self.limits.operations {
            return Err(SourceError::Limit("operations"));
        }
        Ok(())
    }
    fn read_cell<R: RoutingStore>(
        &mut self,
        routing: &mut R,
        at: CellIndex,
    ) -> Result<AnnualCell, CellSourceError<R::Error>> {
        self.charge(1)?;
        if self.failed
            || routing.extent() != self.extent
            || at.raw() != self.next
            || CellIndex::new(at.raw(), self.extent).is_none()
        {
            return Err(
                SourceError::Invalid("failed source, extent, or canonical next cell").into(),
            );
        }
        let (x, y) = self.extent.coordinates(at);
        if y / 512 != self.tile_row {
            self.charge(u64::from(self.domain.columns()))?;
            for window in &mut self.windows {
                *window = None;
            }
            self.tile_row = y / 512;
        }
        self.charge(1)?;
        let record = routing.read(at).map_err(CellSourceError::Routing)?;
        let owner = record
            .owner()
            .filter(|p| CellIndex::new(p.raw(), self.extent).is_some())
            .ok_or(SourceError::Invalid("unresolved terminal owner"))?;
        let receiver = record
            .receiver(self.extent, at)
            .ok_or(SourceError::Invalid("unresolved receiver"))?;
        if record.is_marine() {
            if record.height() > 0
                || owner != at
                || receiver != Receiver::Stop(OutletKind::MarineEntry)
            {
                return Err(SourceError::Invalid("marine terminal authority").into());
            }
            return Ok(AnnualCell {
                at: GlobalCell { x, y },
                bed: HeightMm::new(record.height()),
                rain: RainfallMm::new(0),
                evaporation_um: [0; 12],
                terminal: Terminal::Marine,
            });
        }
        let terminal = if owner == at {
            record
        } else {
            if matches!(receiver, Receiver::Stop(_)) {
                return Err(SourceError::Invalid("terminal points to another owner").into());
            }
            self.charge(1)?;
            routing.read(owner).map_err(CellSourceError::Routing)?
        };
        if terminal.owner() != Some(owner) || (terminal.is_marine() && terminal.height() > 0) {
            return Err(SourceError::Invalid("terminal is not its own owner").into());
        }
        let terminal = match terminal.receiver(self.extent, owner) {
            Some(Receiver::Stop(OutletKind::ClosedDepression)) if !terminal.is_marine() => {
                Terminal::Basin(BasinId(self.extent.anchor_key(owner)))
            }
            Some(Receiver::Stop(OutletKind::MarineEntry)) if terminal.is_marine() => Terminal::Sea,
            Some(Receiver::Stop(OutletKind::DomainExport)) if !terminal.is_marine() => {
                Terminal::DomainExport
            }
            _ => return Err(SourceError::Invalid("invalid receiving terminal").into()),
        };
        self.charge(1)?;
        let prepared = self
            .prepared
            .cell(GlobalCell { x, y })
            .map_err(SourceError::Prepared)?;
        if prepared.height.raw() != record.height() {
            return Err(SourceError::Invalid("prepared/routing physical height").into());
        }
        let column = usize::try_from(x / 512).map_err(|_| SourceError::Limit("column"))?;
        if self.windows[column].is_none() {
            self.charge(WINDOW_WORK)?;
            let area = AreaCoord::new(
                i32::try_from(x / 512).map_err(|_| SourceError::Invalid("area x"))?,
                i32::try_from(y / 512).map_err(|_| SourceError::Invalid("area y"))?,
            );
            self.windows[column] = Some(
                ForcingWindow::for_area(
                    self.grid,
                    self.climate,
                    self.config.latitude_band(),
                    self.forcing_domain,
                    area,
                )
                .map_err(SourceError::Forcing)?,
            );
            self.work.windows += 1;
        }
        self.charge(SAMPLE_WORK)?;
        let annual = temperature_from_base(prepared.temperature_base_centi, prepared.height, false);
        let year = self.windows[column]
            .as_ref()
            .ok_or(SourceError::Invalid("missing cached window"))?
            .sample_with_annual_temperature(GlobalCell { x, y }, annual)
            .map_err(SourceError::Forcing)?;
        self.work.samples += 1;
        Ok(AnnualCell {
            at: GlobalCell { x, y },
            bed: prepared.height,
            rain: prepared.annual_rain,
            evaporation_um: year.periods.map(|m| m.evaporation_um),
            terminal,
        })
    }
    fn success(&mut self) {
        self.next += 1;
        self.work.cells += 1;
    }
    /// Produces one canonical annual aggregation cell; an error permanently stops this pass.
    pub fn cell<R: RoutingStore>(
        &mut self,
        routing: &mut R,
        at: CellIndex,
    ) -> Result<AnnualCell, CellSourceError<R::Error>> {
        if self.mode == Mode::Fine {
            self.failed = true;
            return Err(SourceError::Invalid("mixed source modes").into());
        }
        self.mode = Mode::Annual;
        match self.read_cell(routing, at) {
            Ok(cell) => {
                self.success();
                Ok(cell)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    /// Produces fine-flow input from the same P/A/E calculation. Supply the same
    /// immutable solver-owned leaf_net slice throughout this canonical pass.
    /// Marginal cost is a virtual account debit at the actual terminal, even if wet.
    pub fn fine_cell<R: RoutingStore>(
        &mut self,
        routing: &mut R,
        at: CellIndex,
        leaf_net: &[LeafNet],
    ) -> Result<CellSource, CellSourceError<R::Error>> {
        if self.mode == Mode::Annual {
            self.failed = true;
            return Err(SourceError::Invalid("mixed source modes").into());
        }
        self.mode = Mode::Fine;
        let result = (|| {
            if self.next == 0 {
                if leaf_net.len()
                    > usize::try_from(self.extent.cells())
                        .map_err(|_| SourceError::Limit("leaf count"))?
                {
                    return Err(SourceError::Invalid("too many final leaves").into());
                }
                self.charge(
                    u64::try_from(leaf_net.len()).map_err(|_| SourceError::Limit("leaf count"))?,
                )?;
                if leaf_net.windows(2).any(|w| w[0].leaf >= w[1].leaf)
                    || leaf_net.iter().any(|l| {
                        l.lake.is_some() != l.surface.is_some()
                            || l.lake.is_some_and(|id| id != l.account)
                    })
                {
                    return Err(SourceError::Invalid("final leaf order or level").into());
                }
            }
            let cell = self.read_cell(routing, at)?;
            let b = cell_budget(cell.rain, &cell.evaporation_um);
            let (mut wet, mut marginal) = (None, Litres(0));
            if let Terminal::Basin(id) = cell.terminal {
                let (mut lo, mut hi) = (0, leaf_net.len());
                while lo < hi {
                    self.charge(1)?;
                    let mid = lo + (hi - lo) / 2;
                    if leaf_net[mid].leaf < id {
                        lo = mid + 1;
                    } else {
                        hi = mid;
                    }
                }
                let i = lo;
                if leaf_net.get(i).is_none_or(|l| l.leaf != id) {
                    return Err(SourceError::Invalid("missing final leaf").into());
                }
                let leaf = &leaf_net[i];
                if let (Some(lake), Some(surface)) = (leaf.lake, leaf.surface) {
                    if cell.bed < surface {
                        wet = Some((lake, surface.raw()));
                    }
                }
                if self.extent.anchor_key(at) == id.0 {
                    marginal = leaf.marginal_cost;
                }
            }
            Ok(CellSource {
                at,
                precipitation: b.precipitation,
                land_loss: b.land_loss,
                evaporation: b.evaporation,
                marginal,
                wet,
            })
        })();
        match result {
            Ok(cell) => {
                self.success();
                Ok(cell)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    /// Require exact whole-domain completion before any downstream publication.
    pub fn finish(&self) -> Result<SourceWork, SourceError> {
        if self.failed || self.next != self.extent.cells() {
            return Err(SourceError::Invalid("incomplete source pass"));
        }
        Ok(self.work)
    }
    /// Attempted source work so far, including any surfaced failed query.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn work(&self) -> SourceWork {
        self.work
    }
}
