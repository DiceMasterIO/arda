//! Manifest-first stored-world queries with independent area and block caches.

use arda_core::{
    AreaCoord, AreaObjects, Block, FormatError, GenerateConfig, LoadError, Manifest, SizeKm,
    TerrainFileError, TerrainFileReader, FINE_TERRAIN_LATEST_RECIPE_VERSION, FINE_TERRAIN_PATH,
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

mod area;
pub use area::Area;

/// An immutable stored world; layer caches fill only when queried.
pub struct World {
    dir: PathBuf,
    manifest: Manifest,
    domain: arda_core::hydrology::HydrologyDomain,
    areas: BTreeMap<AreaCoord, OnceLock<Area>>,
    blocks: BTreeMap<AreaCoord, OnceLock<arda_core::BlockArchive>>,
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

        let config = GenerateConfig::new(
            manifest.config.size_km(),
            manifest.config.latitude_band(),
            manifest.config.mean_density_per_km2(),
        )
        .map_err(|e| LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: e.to_string(),
        })?;
        if (manifest.areas_wide, manifest.areas_high) != (config.areas_wide(), config.areas_high())
        {
            return Err(LoadError::ManifestUnreadable {
                dir: dir.display().to_string(),
                reason: "area dimensions disagree with the validated configuration".into(),
            });
        }
        if manifest.fine_terrain.is_some_and(|fine| {
            !(1..=FINE_TERRAIN_LATEST_RECIPE_VERSION).contains(&fine.recipe_version)
        }) {
            return Err(LoadError::ManifestUnreadable {
                dir: dir.display().to_string(),
                reason: "unsupported canonical fine terrain source recipe".into(),
            });
        }
        let invalid_domain = || LoadError::ManifestUnreadable {
            dir: dir.display().to_string(),
            reason: "modeled dimensions overflow".into(),
        };
        let wide = u32::try_from(config.areas_wide()).map_err(|_| invalid_domain())?;
        let high = u32::try_from(config.areas_high()).map_err(|_| invalid_domain())?;
        let domain = arda_core::hydrology::HydrologyDomain {
            width_cells: config
                .size_km()
                .width
                .checked_mul(10)
                .ok_or_else(invalid_domain)?
                .max(wide.checked_mul(512).ok_or_else(invalid_domain)?),
            height_cells: config
                .size_km()
                .height
                .checked_mul(10)
                .ok_or_else(invalid_domain)?
                .max(high.checked_mul(512).ok_or_else(invalid_domain)?),
            exported_areas_wide: wide,
            exported_areas_high: high,
        };
        let areas = config
            .area_coords()
            .map(|at| (at, OnceLock::new()))
            .collect();
        let blocks = config
            .area_coords()
            .map(|at| (at, OnceLock::new()))
            .collect();

        Ok(Self {
            dir: dir.to_path_buf(),
            manifest,
            domain,
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

    /// Opens and verifies the declared canonical fine terrain layer.
    ///
    /// Legacy worlds return `None`. A manifest that declares this layer never
    /// falls back to area heights if the file is missing, malformed, or has a
    /// different geometry. The explicit budget covers the reader's transient
    /// verification and row buffers, not the caller-owned file handle.
    /// # Errors
    /// Returns the named terrain-layer error or rejects insufficient reader RAM.
    pub fn fine_terrain(
        &self,
        max_transient_bytes: u64,
    ) -> Result<Option<TerrainFileReader<File>>, LoadError> {
        if self.manifest.fine_terrain.is_none() {
            return Ok(None);
        }
        let path = self.dir.join(FINE_TERRAIN_PATH);
        let error = |source| LoadError::Corrupt {
            source: FormatError::Terrain {
                path: path.display().to_string(),
                source,
            },
        };
        let file = File::open(&path).map_err(|source| error(TerrainFileError::Io(source)))?;
        let reader = TerrainFileReader::open(file, max_transient_bytes).map_err(error)?;
        let origin = reader.origin();
        let size = self.manifest.config.size_km();
        let required_x = (i64::from(size.width) * 1_000_000_000)
            .max((i64::from(self.domain.width_cells) - 1) * 100_000_000);
        let required_y = (i64::from(size.height) * 1_000_000_000)
            .max((i64::from(self.domain.height_cells) - 1) * 100_000_000);
        let last_x = i128::from(reader.width() - 1) * i128::from(reader.spacing_um());
        let last_y = i128::from(reader.height() - 1) * i128::from(reader.spacing_um());
        if origin.x_um != 0
            || origin.y_um != 0
            || reader.spacing_um() != 39_062_500
            || last_x < i128::from(required_x)
            || last_y < i128::from(required_y)
        {
            return Err(error(TerrainFileError::InvalidHeader(
                "fine terrain geometry does not cover the modeled world",
            )));
        }
        Ok(Some(reader))
    }

    /// Reads and verifies the optional shore layer (logic/02
    /// §fine-formation shore classes): shore classes on the 100 m cell grid and the
    /// island census. Worlds without one (legacy recipes) return `None`.
    ///
    /// # Errors
    /// Returns the named layer error when the file is unreadable or fails
    /// its checksum or structure checks.
    pub fn shore(&self) -> Result<Option<arda_core::ShoreLayer>, LoadError> {
        let path = self.dir.join(arda_core::SHORE_PATH);
        let name = path.display().to_string();
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(LoadError::Corrupt {
                    source: FormatError::Io { path: name, source },
                })
            }
        };
        arda_core::ShoreLayer::decode(&bytes, &name)
            .map(Some)
            .map_err(|source| LoadError::Corrupt { source })
    }

    /// Every area coordinate in this world, row-major.
    pub fn area_coords(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (w, h) = (self.manifest.areas_wide, self.manifest.areas_high);
        (0..h).flat_map(move |y| (0..w).map(move |x| (x, y)))
    }

    /// Reads one area tile.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the tile is outside the continent.
    pub fn area(&self, x: i32, y: i32) -> Result<&Area, LoadError> {
        let slot = self
            .areas
            .get(&AreaCoord::new(x, y))
            .ok_or_else(|| self.area_range(x, y))?;
        if let Some(area) = slot.get() {
            return Ok(area);
        }
        let area = self.read_area(x, y)?;
        Ok(slot.get_or_init(|| area))
    }

    /// Reads an owned area without populating query caches, for bounded exports.
    ///
    /// # Errors
    /// Returns a coordinate error or the named layer's read/format error.
    pub fn read_area(&self, x: i32, y: i32) -> Result<Area, LoadError> {
        let cell_bytes = usize::from(arda_core::AREA_CELLS)
            * usize::from(arda_core::AREA_CELLS)
            * arda_core::formats::cells::CELL_BYTES;
        self.read_area_with_byte_limits(
            x,
            y,
            cell_bytes,
            arda_core::formats::area_objects_v4::ObjectsLimits::default().max_bytes,
        )
    }

    fn read_area_with_byte_limits(
        &self,
        x: i32,
        y: i32,
        cell_bytes: usize,
        object_bytes: usize,
    ) -> Result<Area, LoadError> {
        let at = AreaCoord::new(x, y);
        if !self.areas.contains_key(&at) {
            return Err(self.area_range(x, y));
        }
        let dir = self.dir.join("areas").join(at.dir_name());
        let cells_path = dir.join("cells.bin");
        let cells = arda_core::decode_cells(
            &cells_path.display().to_string(),
            &read_bounded(&cells_path, cell_bytes)?,
        )?;
        let objects = self.read_area_objects_with_byte_limit(x, y, object_bytes)?;
        let water = self.read_area_water_for(x, y, &objects)?;
        Ok(Area {
            cells,
            objects,
            water,
        })
    }

    /// Reads a neighboring area's stored water forms, when the world has them.
    pub(crate) fn read_area_water(
        &self,
        x: i32,
        y: i32,
    ) -> Result<Option<arda_core::water::AreaWater>, LoadError> {
        let objects = self.read_area_objects(x, y)?;
        self.read_area_water_for(x, y, &objects)
    }

    /// Optional `water.bin`, checked against the area's rivers and lakes.
    fn read_area_water_for(
        &self,
        x: i32,
        y: i32,
        objects: &AreaObjects,
    ) -> Result<Option<arda_core::water::AreaWater>, LoadError> {
        let water_path = self
            .dir
            .join("areas")
            .join(AreaCoord::new(x, y).dir_name())
            .join("water.bin");
        if !water_path.exists() {
            return Ok(None);
        }
        let w = arda_core::decode_water(
            &read_bounded(&water_path, WATER_BYTES)?,
            &water_path.display().to_string(),
        )?;
        if w.segments.len() != objects.rivers.len() || w.lakes.len() != objects.lakes.len() {
            return Err(FormatError::UnexpectedEof {
                path: water_path.display().to_string(),
                read: w.segments.len() + w.lakes.len(),
                expected: objects.rivers.len() + objects.lakes.len(),
            }
            .into());
        }
        Ok(Some(w))
    }

    /// Reads only bounded saved object context for a neighboring Atlas overview area.
    pub(crate) fn read_area_objects(&self, x: i32, y: i32) -> Result<AreaObjects, LoadError> {
        self.read_area_objects_with_byte_limit(
            x,
            y,
            arda_core::formats::area_objects_v4::ObjectsLimits::default().max_bytes,
        )
    }

    fn read_area_objects_with_byte_limit(
        &self,
        x: i32,
        y: i32,
        object_bytes: usize,
    ) -> Result<AreaObjects, LoadError> {
        let at = AreaCoord::new(x, y);
        if !self.areas.contains_key(&at) {
            return Err(self.area_range(x, y));
        }
        let objects_path = self
            .dir
            .join("areas")
            .join(at.dir_name())
            .join("objects.bin");
        let objects = arda_core::decode_objects(
            &objects_path.display().to_string(),
            &read_bounded(&objects_path, object_bytes)?,
        )?;
        arda_core::formats::hydrology::validate_area_context_domain(&objects.global, self.domain)
            .map_err(|source| hydrology_error(&objects_path, source))?;
        if objects.channel_edges.iter().any(|edge| {
            [edge.from, edge.to]
                .iter()
                .any(|at| at.x >= self.domain.width_cells || at.y >= self.domain.height_cells)
        }) {
            return Err(hydrology_error(
                &objects_path,
                arda_core::formats::hydrology::HydrologyFormatError::Invalid(
                    "saved channel leaves modeled domain",
                ),
            ));
        }
        Ok(objects)
    }

    fn area_range(&self, x: i32, y: i32) -> LoadError {
        LoadError::OutOfRange {
            what: "area",
            x,
            y,
            max_x: self.manifest.areas_wide - 1,
            max_y: self.manifest.areas_high - 1,
        }
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
        let slot = self
            .blocks
            .get(&AreaCoord::new(ax, ay))
            .ok_or_else(|| self.area_range(ax, ay))?;
        let at = arda_core::CellCoord::new(cx, cy).ok_or_else(out_of_range)?;
        let archive = if let Some(archive) = slot.get() {
            archive
        } else {
            let path = self
                .dir
                .join("blocks")
                .join(format!("{}.tiles.zst", AreaCoord::new(ax, ay).dir_name()));
            let loaded = arda_core::decode_blocks(&path.display().to_string(), &read(&path)?)?;
            slot.get_or_init(|| loaded)
        };
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

fn hydrology_error(
    path: &Path,
    source: arda_core::formats::hydrology::HydrologyFormatError,
) -> LoadError {
    arda_core::FormatError::Hydrology {
        path: path.display().to_string(),
        source,
    }
    .into()
}

/// Largest water-forms layer read, bytes.
const WATER_BYTES: usize = 64 << 20;

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, LoadError> {
    let io_error = |source| arda_core::FormatError::Io {
        path: path.display().to_string(),
        source,
    };
    let exceeded = || {
        hydrology_error(
            path,
            arda_core::formats::hydrology::HydrologyFormatError::Limit("saved layer byte limit"),
        )
    };
    let mut file = std::fs::File::open(path).map_err(io_error)?;
    let length =
        usize::try_from(file.metadata().map_err(io_error)?.len()).map_err(|_| exceeded())?;
    if length > limit {
        return Err(exceeded());
    }
    // Allocate only the admitted metadata length; detect growth with a stack byte.
    let mut bytes = vec![0; length];
    file.read_exact(&mut bytes).map_err(io_error)?;
    if file.read(&mut [0]).map_err(io_error)? != 0 {
        return Err(hydrology_error(
            path,
            arda_core::formats::hydrology::HydrologyFormatError::Invalid(
                "saved layer changed length while reading",
            ),
        ));
    }
    Ok(bytes)
}

fn read(path: &Path) -> Result<Vec<u8>, LoadError> {
    std::fs::read(path).map_err(|source| {
        arda_core::FormatError::Io {
            path: path.display().to_string(),
            source,
        }
        .into()
    })
}

#[cfg(test)]
mod tests;
