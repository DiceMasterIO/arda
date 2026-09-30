//! Read-only access to the stored world: fine-terrain nodes and saved cells,
//! only through the public `arda` APIs, so the layer composes with any world
//! that stores a recipe-5 fine field.

use crate::MidzoomError;
use arda::World;
use arda_core::hydrology::ChannelEdge;
use arda_core::{Cover, TerrainFileReader, TerrainKind, TerrainPoint, AREA_CELLS};
use std::collections::HashMap;
use std::fs::File;
use std::sync::{Arc, Mutex};

/// Stored fine-lattice spacing, micrometres (39.0625 m).
pub const FINE_UM: i64 = 39_062_500;
/// Saved cell edge, micrometres.
pub const CELL_UM: i64 = 100_000_000;

/// The saved facts refinement reads from one 100 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellInfo {
    /// Sea, land or lake.
    pub terrain: TerrainKind,
    /// Dominant ground cover.
    pub cover: Cover,
    /// Channel width in decimetres; 0 without a watercourse.
    pub river_width_dm: u32,
}

/// Stored terrain as refinement sees it.
pub trait Terrain: Sync {
    /// World seed (detail phases derive from it).
    fn seed(&self) -> u64;
    /// Stored lattice size in nodes `(wide, high)`.
    fn nodes(&self) -> (i64, i64);
    /// Stored heights (mm) of nodes `[kx0, kx0 + w) × [ky0, ky0 + h)`,
    /// row-major; nodes outside the lattice repeat its nearest edge node.
    ///
    /// # Errors
    /// The layer failed to read or the window could not be allocated.
    fn read_nodes(&self, kx0: i64, ky0: i64, w: usize, h: usize) -> Result<Vec<i32>, MidzoomError>;
    /// The saved cell `(gx, gy)`, clamped into the world.
    ///
    /// # Errors
    /// The area layer failed to read.
    fn cell(&self, gx: i64, gy: i64) -> Result<CellInfo, MidzoomError>;
}

type ChunkCache = HashMap<(i64, i64), Arc<Vec<i32>>>;
type AreaCache = HashMap<(i32, i32), Arc<AreaFacts>>;

const CHUNK: i64 = 128;
const MAX_CHUNKS: usize = 512;
const MAX_AREAS: usize = 16;

/// The saved facts of one area that refinement and rivers read (≈ 2 MB).
#[derive(Debug, Clone)]
pub struct AreaFacts {
    /// Cells, row-major, 512 × 512.
    pub cells: Vec<CellInfo>,
    /// Saved channel centreline edges touching the area.
    pub edges: Vec<ChannelEdge>,
}

/// A stored [`World`] as a [`Terrain`]: the verified fine layer read in
/// 128-node chunks and cached (pure reads only, so caching never changes
/// an answer), saved cells through the world's own area cache.
pub struct WorldTerrain {
    world: Arc<World>,
    reader: Mutex<TerrainFileReader<File>>,
    wide: i64,
    high: i64,
    chunks: Mutex<ChunkCache>,
    areas: Mutex<AreaCache>,
}

impl std::fmt::Debug for WorldTerrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldTerrain")
            .field("nodes", &(self.wide, self.high))
            .finish_non_exhaustive()
    }
}

impl WorldTerrain {
    /// Opens the world's fine terrain layer.
    ///
    /// # Errors
    /// [`MidzoomError::NoFineTerrain`] for worlds without one, or a read error.
    pub fn new(world: Arc<World>) -> Result<Self, MidzoomError> {
        let reader = world
            .fine_terrain(1 << 20)?
            .ok_or(MidzoomError::NoFineTerrain)?;
        let (wide, high) = (i64::from(reader.width()), i64::from(reader.height()));
        Ok(Self {
            world,
            reader: Mutex::new(reader),
            wide,
            high,
            chunks: Mutex::new(HashMap::new()),
            areas: Mutex::new(HashMap::new()),
        })
    }

    /// The wrapped world.
    #[must_use]
    pub fn world(&self) -> &Arc<World> {
        &self.world
    }

    /// The saved facts of area `(ax, ay)`, read once into a bounded cache.
    ///
    /// # Errors
    /// The area is outside the world or failed to read.
    pub fn area(&self, ax: i32, ay: i32) -> Result<Arc<AreaFacts>, MidzoomError> {
        if let Some(a) = self
            .areas
            .lock()
            .map_err(|_| MidzoomError::Poisoned)?
            .get(&(ax, ay))
        {
            return Ok(Arc::clone(a));
        }
        let area = self.world.read_area(ax, ay)?;
        let n = AREA_CELLS;
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(usize::from(n) * usize::from(n))
            .map_err(|_| MidzoomError::ResourceLimit("area facts"))?;
        for y in 0..n {
            for x in 0..n {
                let c = area.cell(x, y)?;
                cells.push(CellInfo {
                    terrain: c.terrain,
                    cover: c.cover,
                    river_width_dm: c.watercourse_width_dm,
                });
            }
        }
        let facts = Arc::new(AreaFacts {
            cells,
            edges: area.channel_edges().to_vec(),
        });
        let mut cache = self.areas.lock().map_err(|_| MidzoomError::Poisoned)?;
        if cache.len() >= MAX_AREAS {
            cache.clear();
        }
        cache.insert((ax, ay), Arc::clone(&facts));
        Ok(facts)
    }

    fn chunk(&self, cx: i64, cy: i64) -> Result<Arc<Vec<i32>>, MidzoomError> {
        if let Some(c) = self
            .chunks
            .lock()
            .map_err(|_| MidzoomError::Poisoned)?
            .get(&(cx, cy))
        {
            return Ok(Arc::clone(c));
        }
        let mut data = Vec::new();
        data.try_reserve_exact(usize::try_from(CHUNK * CHUNK).unwrap_or(0))
            .map_err(|_| MidzoomError::ResourceLimit("fine chunk"))?;
        {
            let mut reader = self.reader.lock().map_err(|_| MidzoomError::Poisoned)?;
            for y in cy * CHUNK..(cy + 1) * CHUNK {
                for x in cx * CHUNK..(cx + 1) * CHUNK {
                    let point = TerrainPoint {
                        x_um: x.clamp(0, self.wide - 1) * FINE_UM,
                        y_um: y.clamp(0, self.high - 1) * FINE_UM,
                    };
                    let h = reader.sample(point)?.map_or(0, arda_core::HeightMm::raw);
                    data.push(h);
                }
            }
        }
        let chunk = Arc::new(data);
        let mut cache = self.chunks.lock().map_err(|_| MidzoomError::Poisoned)?;
        if cache.len() >= MAX_CHUNKS {
            cache.clear();
        }
        cache.insert((cx, cy), Arc::clone(&chunk));
        Ok(chunk)
    }
}

impl Terrain for WorldTerrain {
    fn seed(&self) -> u64 {
        self.world.seed()
    }

    fn nodes(&self) -> (i64, i64) {
        (self.wide, self.high)
    }

    fn read_nodes(&self, kx0: i64, ky0: i64, w: usize, h: usize) -> Result<Vec<i32>, MidzoomError> {
        let mut out = Vec::new();
        out.try_reserve_exact(w * h)
            .map_err(|_| MidzoomError::ResourceLimit("fine window"))?;
        out.resize(w * h, 0);
        let (w64, h64) = (
            i64::try_from(w).map_err(|_| MidzoomError::Window("width".into()))?,
            i64::try_from(h).map_err(|_| MidzoomError::Window("height".into()))?,
        );
        let clamp = |v: i64, n: i64| v.clamp(0, n - 1);
        // Visit every chunk the clamped window touches once.
        let (cx0, cx1) = (
            clamp(kx0, self.wide).div_euclid(CHUNK),
            clamp(kx0 + w64 - 1, self.wide).div_euclid(CHUNK),
        );
        let (cy0, cy1) = (
            clamp(ky0, self.high).div_euclid(CHUNK),
            clamp(ky0 + h64 - 1, self.high).div_euclid(CHUNK),
        );
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let chunk = self.chunk(cx, cy)?;
                for (j, row) in out.chunks_mut(w).enumerate() {
                    let y = clamp(ky0 + i64::try_from(j).unwrap_or(0), self.high);
                    if y.div_euclid(CHUNK) != cy {
                        continue;
                    }
                    for (i, v) in row.iter_mut().enumerate() {
                        let x = clamp(kx0 + i64::try_from(i).unwrap_or(0), self.wide);
                        if x.div_euclid(CHUNK) != cx {
                            continue;
                        }
                        let at = (y - cy * CHUNK) * CHUNK + (x - cx * CHUNK);
                        *v = usize::try_from(at)
                            .ok()
                            .and_then(|a| chunk.get(a))
                            .copied()
                            .unwrap_or(0);
                    }
                }
            }
        }
        Ok(out)
    }

    fn cell(&self, gx: i64, gy: i64) -> Result<CellInfo, MidzoomError> {
        let m = self.world.manifest();
        let n = i64::from(AREA_CELLS);
        let gx = gx.clamp(0, i64::from(m.areas_wide) * n - 1);
        let gy = gy.clamp(0, i64::from(m.areas_high) * n - 1);
        let area = self.area(
            i32::try_from(gx / n).unwrap_or(0),
            i32::try_from(gy / n).unwrap_or(0),
        )?;
        let at = usize::try_from((gy % n) * n + gx % n).unwrap_or(0);
        area.cells
            .get(at)
            .copied()
            .ok_or_else(|| MidzoomError::Window("cell outside area".into()))
    }
}

/// An in-memory [`Terrain`] for tests and synthetic studies.
#[derive(Debug, Clone)]
pub struct GridTerrain {
    /// Seed.
    pub seed: u64,
    /// Lattice width in nodes.
    pub wide: i64,
    /// Lattice height in nodes.
    pub high: i64,
    /// Heights, row-major, mm.
    pub heights: Vec<i32>,
    /// Cell returned everywhere.
    pub cell: CellInfo,
}

impl GridTerrain {
    /// A `wide × high` lattice with heights from `f(kx, ky)` (mm) and one
    /// land cell everywhere.
    #[must_use]
    pub fn from_fn(seed: u64, wide: i64, high: i64, f: impl Fn(i64, i64) -> i32) -> Self {
        let heights = (0..high)
            .flat_map(|y| (0..wide).map(move |x| (x, y)))
            .map(|(x, y)| f(x, y))
            .collect();
        Self {
            seed,
            wide,
            high,
            heights,
            cell: CellInfo {
                terrain: TerrainKind::Land,
                cover: Cover::Rock,
                river_width_dm: 0,
            },
        }
    }
}

impl Terrain for GridTerrain {
    fn seed(&self) -> u64 {
        self.seed
    }

    fn nodes(&self) -> (i64, i64) {
        (self.wide, self.high)
    }

    fn read_nodes(&self, kx0: i64, ky0: i64, w: usize, h: usize) -> Result<Vec<i32>, MidzoomError> {
        let mut out = Vec::with_capacity(w * h);
        for j in 0..h {
            for i in 0..w {
                let x = (kx0 + i64::try_from(i).unwrap_or(0)).clamp(0, self.wide - 1);
                let y = (ky0 + i64::try_from(j).unwrap_or(0)).clamp(0, self.high - 1);
                let at = usize::try_from(y * self.wide + x).unwrap_or(0);
                out.push(self.heights.get(at).copied().unwrap_or(0));
            }
        }
        Ok(out)
    }

    fn cell(&self, _gx: i64, _gy: i64) -> Result<CellInfo, MidzoomError> {
        Ok(self.cell)
    }
}
