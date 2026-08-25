//! The public facade for arda. Consumers depend on this crate only.
//!
//! Mirrors `mockup/04`'s transcript: `World` (manifest) to `Area` (cells and
//! objects) to `Cell` / `Block`.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub use arda_core::{
    AreaCells, AreaObjects, Block, Cell, Cover, GenerateConfig, Lake, LatitudeBand, LoadError,
    Manifest, RiverSegment, RoadClass, SizeKm, TerrainKind, TileId, ValidationStats,
};
pub use arda_gen::GenError;

/// Runs the batch, writing a world directory (`mockup/01`).
///
/// # Errors
/// See [`GenError`].
pub fn generate(seed: u64, config: GenerateConfig, out: &Path) -> Result<Manifest, GenError> {
    arda_gen::generate_world(seed, config, out)
}

/// Output format for `export` (`mockup/03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Cartographic PNG.
    Png,
    /// Versioned JSON.
    Json,
}

/// An export failure at the facade boundary.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The world could not supply the requested tile.
    #[error(transparent)]
    Load(#[from] LoadError),
    /// The renderer refused.
    #[error(transparent)]
    Render(#[from] arda_render::RenderError),
    /// The artifact could not be written.
    #[error("failed writing {path}: {source}")]
    Write {
        /// The file being written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
}

/// Exports one area tile to `out`, returning the file written.
///
/// # Errors
/// See [`ExportError`].
pub fn export_area(
    world: &World,
    ax: i32,
    ay: i32,
    out: &Path,
    format: ExportFormat,
) -> Result<PathBuf, ExportError> {
    let area = world.area(ax, ay)?;
    let name = format!("area_{ax:02}_{ay:02}");
    let (path, bytes) = match format {
        ExportFormat::Png => (
            out.join(format!("{name}.png")),
            arda_render::render_area_png(area.cells())?,
        ),
        ExportFormat::Json => (
            out.join(format!("{name}.json")),
            arda_render::area_json(world.manifest(), ax, ay, area.cells(), area.objects())
                .into_bytes(),
        ),
    };
    std::fs::write(&path, bytes).map_err(|e| ExportError::Write {
        path: path.display().to_string(),
        source: e,
    })?;
    Ok(path)
}

/// One loaded area tile.
pub struct Area {
    cells: AreaCells,
    objects: AreaObjects,
}

impl Area {
    /// Reads one cell.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the coordinates leave the tile.
    pub fn cell(&self, x: u16, y: u16) -> Result<&Cell, LoadError> {
        let at = arda_core::CellCoord::new(x, y).ok_or(LoadError::OutOfRange {
            what: "cell",
            x: i32::from(x),
            y: i32::from(y),
            max_x: i32::from(arda_core::AREA_CELLS) - 1,
            max_y: i32::from(arda_core::AREA_CELLS) - 1,
        })?;
        Ok(self.cells.get(at))
    }

    /// River segments in this tile.
    #[must_use]
    pub fn rivers(&self) -> &[RiverSegment] {
        &self.objects.rivers
    }

    /// Lakes in this tile.
    #[must_use]
    pub fn lakes(&self) -> &[Lake] {
        &self.objects.lakes
    }

    /// The raw cell grid, for renderers.
    #[must_use]
    pub const fn cells(&self) -> &AreaCells {
        &self.cells
    }

    /// The raw object lists, for renderers.
    #[must_use]
    pub const fn objects(&self) -> &AreaObjects {
        &self.objects
    }
}

impl std::fmt::Debug for Area {
    /// Summary only — an area holds 262,144 cells, so the full grid is never
    /// formatted.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Area")
            .field("rivers", &self.objects.rivers.len())
            .field("lakes", &self.objects.lakes.len())
            .finish_non_exhaustive()
    }
}

/// A loaded world (`logic/05`).
///
/// `ponytail:` areas and blocks are read eagerly at `load`. `logic/05` wants
/// them lazy with an O(accessed) cache; the upgrade is to keep the directory
/// path and populate these maps on first access behind a `OnceLock`. The
/// skeleton's eight tiles fit in memory, so laziness is not yet earned.
pub struct World {
    dir: PathBuf,
    manifest: Manifest,
    areas: BTreeMap<(i32, i32), Area>,
    blocks: BTreeMap<(i32, i32), arda_core::BlockArchive>,
}

impl World {
    /// Opens a generated world.
    ///
    /// # Errors
    /// [`LoadError::ManifestMissing`] for a partial world,
    /// [`LoadError::VersionSkew`] for an incompatible format,
    /// [`LoadError::Corrupt`] for a bad layer.
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let manifest = arda_core::read_manifest(dir)?;

        let mut areas = BTreeMap::new();
        let mut blocks = BTreeMap::new();
        for ay in 0..manifest.areas_high {
            for ax in 0..manifest.areas_wide {
                let name = arda_core::AreaCoord::new(ax, ay).dir_name();

                let cells_path = dir.join("areas").join(&name).join("cells.bin");
                let cells = arda_core::decode_cells(
                    &cells_path.display().to_string(),
                    &read(&cells_path)?,
                )?;

                let obj_path = dir.join("areas").join(&name).join("objects.bin");
                let objects =
                    arda_core::decode_objects(&obj_path.display().to_string(), &read(&obj_path)?)?;

                areas.insert((ax, ay), Area { cells, objects });

                let blk_path = dir.join("blocks").join(format!("{name}.tiles.zst"));
                blocks.insert(
                    (ax, ay),
                    arda_core::decode_blocks(&blk_path.display().to_string(), &read(&blk_path)?)?,
                );
            }
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
            areas,
            blocks,
        })
    }

    /// The seed this world was generated from.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.manifest.seed
    }

    /// Continent extent.
    #[must_use]
    pub const fn size_km(&self) -> SizeKm {
        self.manifest.config.size_km()
    }

    /// Number of area tiles.
    #[must_use]
    pub fn areas(&self) -> usize {
        self.areas.len()
    }

    /// The directory this world was loaded from.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The manifest.
    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Reads one area tile.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the tile is outside the continent.
    pub fn area(&self, x: i32, y: i32) -> Result<&Area, LoadError> {
        self.areas.get(&(x, y)).ok_or(LoadError::OutOfRange {
            what: "area",
            x,
            y,
            max_x: self.manifest.areas_wide - 1,
            max_y: self.manifest.areas_high - 1,
        })
    }

    /// Reads one block.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the tile or cell is outside the world,
    /// or no block was materialised for that cell.
    pub fn block(&self, ax: i32, ay: i32, cx: u16, cy: u16) -> Result<&Block, LoadError> {
        let out_of_range = || LoadError::OutOfRange {
            what: "block",
            x: i32::from(cx),
            y: i32::from(cy),
            max_x: i32::from(arda_core::AREA_CELLS) - 1,
            max_y: i32::from(arda_core::AREA_CELLS) - 1,
        };
        let archive = self.blocks.get(&(ax, ay)).ok_or(LoadError::OutOfRange {
            what: "area",
            x: ax,
            y: ay,
            max_x: self.manifest.areas_wide - 1,
            max_y: self.manifest.areas_high - 1,
        })?;
        let at = arda_core::CellCoord::new(cx, cy).ok_or_else(out_of_range)?;
        archive.get(at).ok_or_else(out_of_range)
    }
}

impl std::fmt::Debug for World {
    /// Summary only — see [`Area`]'s note.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("dir", &self.dir)
            .field("seed", &self.manifest.seed)
            .field("areas", &self.areas.len())
            .finish_non_exhaustive()
    }
}

fn read(path: &Path) -> Result<Vec<u8>, LoadError> {
    std::fs::read(path).map_err(|e| LoadError::ManifestUnreadable {
        dir: path.display().to_string(),
        reason: e.to_string(),
    })
}
