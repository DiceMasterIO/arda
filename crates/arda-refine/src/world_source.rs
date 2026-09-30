//! [`Source`] over a stored [`arda::World`].
//!
//! Areas load lazily through the world's own cache; per-area lake and
//! channel-edge indexes and fine-terrain samples are cached here. Caches only
//! memoise pure reads, so they can never change an answer.

use crate::error::RefineError;
use crate::source::{CellKey, Edge, LakeInfo, Source};
use arda::{Cell, World};
use arda_core::{TerrainFileReader, TerrainPoint, AREA_CELLS};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::sync::{Arc, Mutex};

/// Fine terrain lattice spacing in micrometres.
const FINE_SPACING_UM: i64 = 39_062_500;

#[derive(Default)]
struct AreaIndex {
    lakes: BTreeMap<CellKey, LakeInfo>,
    edges: BTreeMap<CellKey, Vec<Edge>>,
}

struct Fine {
    reader: TerrainFileReader<File>,
    cache: HashMap<(i64, i64), i32>,
}

/// A borrowed or shared world.
enum WorldRef<'w> {
    Borrowed(&'w World),
    Shared(Arc<World>),
}

impl WorldRef<'_> {
    fn get(&self) -> &World {
        match self {
            Self::Borrowed(w) => w,
            Self::Shared(w) => w,
        }
    }
}

/// A stored world as a refinement source.
pub struct WorldSource<'w> {
    world: WorldRef<'w>,
    wide: i64,
    high: i64,
    index: Mutex<BTreeMap<(i32, i32), Arc<AreaIndex>>>,
    fine: Mutex<Option<Fine>>,
}

impl std::fmt::Debug for WorldSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldSource")
            .field("cells", &(self.wide, self.high))
            .finish_non_exhaustive()
    }
}

impl WorldSource<'static> {
    /// Wraps a shared world, for long-lived owners such as a server.
    ///
    /// # Errors
    /// The fine terrain layer is declared but unreadable.
    pub fn shared(world: Arc<World>) -> Result<Self, RefineError> {
        Self::wrap(WorldRef::Shared(world))
    }
}

impl<'w> WorldSource<'w> {
    /// Wraps a loaded world, opening its fine terrain layer when declared.
    ///
    /// # Errors
    /// The fine terrain layer is declared but unreadable.
    pub fn new(world: &'w World) -> Result<Self, RefineError> {
        Self::wrap(WorldRef::Borrowed(world))
    }

    fn wrap(world: WorldRef<'w>) -> Result<Self, RefineError> {
        let w = world.get();
        let m = w.manifest();
        let (wide, high) = (
            i64::from(m.areas_wide) * i64::from(AREA_CELLS),
            i64::from(m.areas_high) * i64::from(AREA_CELLS),
        );
        let fine = w.fine_terrain(1 << 20)?.map(|reader| Fine {
            reader,
            cache: HashMap::new(),
        });
        Ok(Self {
            world,
            wide,
            high,
            index: Mutex::new(BTreeMap::new()),
            fine: Mutex::new(fine),
        })
    }

    /// The wrapped world.
    #[must_use]
    pub fn world(&self) -> &World {
        self.world.get()
    }

    fn split(&self, at: CellKey) -> Result<(i32, i32, u16, u16), RefineError> {
        let c = self.clamp(at);
        let n = i64::from(AREA_CELLS);
        let bad = || RefineError::OutOfWorld {
            what: "cell",
            x: at.x,
            y: at.y,
        };
        Ok((
            i32::try_from(c.x / n).map_err(|_| bad())?,
            i32::try_from(c.y / n).map_err(|_| bad())?,
            u16::try_from(c.x % n).map_err(|_| bad())?,
            u16::try_from(c.y % n).map_err(|_| bad())?,
        ))
    }

    fn area_index(&self, ax: i32, ay: i32) -> Result<Arc<AreaIndex>, RefineError> {
        {
            let guard = self.index.lock().map_err(|_| RefineError::Poisoned)?;
            if let Some(ix) = guard.get(&(ax, ay)) {
                return Ok(Arc::clone(ix));
            }
        }
        let area = self.world.get().area(ax, ay)?;
        let origin = (i64::from(ax) * 512, i64::from(ay) * 512);
        let mut ix = AreaIndex::default();
        for lake in area.lakes() {
            let info = LakeInfo {
                surface_mm: lake.surface.raw(),
                depth_mm: lake.depth_mm,
            };
            for c in &lake.cells {
                let key = CellKey::new(origin.0 + i64::from(c.x()), origin.1 + i64::from(c.y()));
                ix.lakes.insert(key, info);
            }
        }
        for e in area.channel_edges() {
            let edge = Edge {
                from: CellKey::new(i64::from(e.from.x), i64::from(e.from.y)),
                to: CellKey::new(i64::from(e.to.x), i64::from(e.to.y)),
                from_width_dm: e.from_width_dm,
                to_width_dm: e.to_width_dm,
                discharge_milli: e.discharge.raw(),
            };
            for key in [edge.from, edge.to] {
                ix.edges.entry(key).or_default().push(edge);
            }
        }
        for list in ix.edges.values_mut() {
            list.sort();
            list.dedup();
        }
        let ix = Arc::new(ix);
        let mut guard = self.index.lock().map_err(|_| RefineError::Poisoned)?;
        Ok(Arc::clone(guard.entry((ax, ay)).or_insert(ix)))
    }
}

impl Source for WorldSource<'_> {
    fn seed(&self) -> u64 {
        self.world.get().seed()
    }

    fn cells_wide_high(&self) -> (i64, i64) {
        (self.wide, self.high)
    }

    fn cell(&self, at: CellKey) -> Result<Cell, RefineError> {
        let (ax, ay, x, y) = self.split(at)?;
        Ok(*self.world.get().area(ax, ay)?.cell(x, y)?)
    }

    fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> {
        let (ax, ay, _, _) = self.split(at)?;
        Ok(self.area_index(ax, ay)?.lakes.get(&self.clamp(at)).copied())
    }

    fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
        if self.clamp(at) != at {
            return Ok(Vec::new());
        }
        let (ax, ay, _, _) = self.split(at)?;
        Ok(self
            .area_index(ax, ay)?
            .edges
            .get(&at)
            .cloned()
            .unwrap_or_default())
    }

    fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
        let mut guard = self.fine.lock().map_err(|_| RefineError::Poisoned)?;
        let Some(fine) = guard.as_mut() else {
            return Ok(None);
        };
        let kx = kx.clamp(0, i64::from(fine.reader.width()) - 1);
        let ky = ky.clamp(0, i64::from(fine.reader.height()) - 1);
        if let Some(v) = fine.cache.get(&(kx, ky)) {
            return Ok(Some(*v));
        }
        let point = TerrainPoint {
            x_um: kx * FINE_SPACING_UM,
            y_um: ky * FINE_SPACING_UM,
        };
        let v = fine.reader.sample(point)?.map(|h| h.raw());
        if let Some(v) = v {
            fine.cache.insert((kx, ky), v);
        }
        Ok(v)
    }
}
