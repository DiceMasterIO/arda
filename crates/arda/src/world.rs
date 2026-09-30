//! Manifest-first stored-world queries with independent area and block caches.

use arda_core::{
    AreaCells, AreaCoord, AreaObjects, Block, Cell, FormatError, GenerateConfig, Lake, LoadError,
    Manifest, RiverSegment, SizeKm, TerrainFileError, TerrainFileReader, FINE_TERRAIN_PATH,
    FINE_TERRAIN_RECIPE_VERSION,
};
use std::{
    collections::BTreeMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::OnceLock,
};

/// One loaded area tile.
pub struct Area {
    cells: AreaCells,
    objects: AreaObjects,
    water: Option<arda_core::water::AreaWater>,
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

    /// Saved physical channel centreline edges touching this tile.
    #[must_use]
    pub fn channel_edges(&self) -> &[arda_core::hydrology::ChannelEdge] {
        &self.objects.channel_edges
    }

    /// The raw cell grid, for renderers.
    #[must_use]
    pub const fn cells(&self) -> &AreaCells {
        &self.cells
    }

    /// Transfers the cell grid to an uncached streaming overview export.
    pub(crate) fn into_cells(self) -> AreaCells {
        self.cells
    }

    /// Transfers the cells and saved objects to the Atlas overview exporter.
    pub(crate) fn into_render_parts(self) -> (AreaCells, AreaObjects) {
        (self.cells, self.objects)
    }

    /// The raw object lists, for renderers.
    #[must_use]
    pub const fn objects(&self) -> &AreaObjects {
        &self.objects
    }

    /// Stored river and lake forms (logic/02 §world-water): per river
    /// segment and lake, in the order of [`Self::rivers`] and
    /// [`Self::lakes`]. `None` for worlds whose recipe does not publish
    /// them.
    #[must_use]
    pub const fn water(&self) -> Option<&arda_core::water::AreaWater> {
        self.water.as_ref()
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
        if manifest
            .fine_terrain
            .is_some_and(|fine| !(1..=FINE_TERRAIN_RECIPE_VERSION).contains(&fine.recipe_version))
        {
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
mod tests {
    use super::*;
    use arda_core::{
        FineTerrainDescriptor, HeightMm, TerrainKind, ValidationStats, FORMAT_VERSION,
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            use std::sync::atomic::{AtomicU32, Ordering};
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let path = std::env::temp_dir().join(format!(
                "arda-lazy-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            let manifest = Manifest {
                format_version: FORMAT_VERSION,
                arda_version: "0.1.0".into(),
                seed: 42,
                config: GenerateConfig::MICRO,
                areas_wide: 2,
                areas_high: 4,
                stats: ValidationStats {
                    land_fraction_permille: 500,
                    area_count: 8,
                    settlement_count: 0,
                    named_river_count: 0,
                    river_count: 0,
                },
                fine_terrain: None,
            };
            arda_core::write_manifest(&path, &manifest).unwrap();
            Self(path)
        }
        fn write_area(&self, x: i32, y: i32) {
            let path = self.0.join("areas").join(AreaCoord::new(x, y).dir_name());
            std::fs::create_dir_all(&path).unwrap();
            let cells = AreaCells::flat(Cell {
                terrain: TerrainKind::Land,
                height: HeightMm::new(1234),
                ..Cell::default()
            });
            std::fs::write(path.join("cells.bin"), arda_core::encode_cells(&cells)).unwrap();
            std::fs::write(
                path.join("objects.bin"),
                arda_core::encode_objects(&AreaObjects::empty()).unwrap(),
            )
            .unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn manifest_load_does_not_read_any_layer() {
        let fixture = Fixture::new();
        let world = World::load(&fixture.0).unwrap();
        assert_eq!(world.areas(), 8);
        assert_eq!(
            world.area_coords().collect::<Vec<_>>(),
            [
                (0, 0),
                (1, 0),
                (0, 1),
                (1, 1),
                (0, 2),
                (1, 2),
                (0, 3),
                (1, 3)
            ]
        );
        let error = world.area(1, 2).unwrap_err();
        assert!(matches!(
            error,
            LoadError::Corrupt {
                source: arda_core::FormatError::Io { path, .. }
            } if Path::new(&path) == fixture.0.join("areas").join("01_02").join("cells.bin")
        ));
        assert!(matches!(
            world.area(-1, 0),
            Err(LoadError::OutOfRange { what: "area", .. })
        ));
    }

    #[test]
    fn declared_fine_layer_never_falls_back_when_missing_or_corrupt() {
        let fixture = Fixture::new();
        let legacy = World::load(&fixture.0).unwrap();
        assert!(legacy.fine_terrain(1 << 20).unwrap().is_none());

        let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
        manifest.fine_terrain = Some(FineTerrainDescriptor {
            recipe_version: FINE_TERRAIN_RECIPE_VERSION,
            attempt: 0,
        });
        arda_core::write_manifest(&fixture.0, &manifest).unwrap();
        let world = World::load(&fixture.0).unwrap();
        assert!(matches!(
            world.fine_terrain(1 << 20),
            Err(LoadError::Corrupt {
                source: FormatError::Terrain { .. }
            })
        ));
        let path = fixture.0.join(FINE_TERRAIN_PATH);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"not a fine terrain").unwrap();
        assert!(matches!(
            world.fine_terrain(1 << 20),
            Err(LoadError::Corrupt {
                source: FormatError::Terrain { .. }
            })
        ));

        manifest.fine_terrain.as_mut().unwrap().recipe_version += 1;
        arda_core::write_manifest(&fixture.0, &manifest).unwrap();
        assert!(matches!(
            World::load(&fixture.0),
            Err(LoadError::ManifestUnreadable { .. })
        ));
    }

    #[test]
    fn owned_export_reads_do_not_fill_borrowed_cache() {
        let fixture = Fixture::new();
        fixture.write_area(0, 0);
        let world = World::load(&fixture.0).unwrap();
        let owned = world.read_area(0, 0).unwrap();
        assert_eq!(owned.cell(0, 0).unwrap().height.raw(), 1234);
        std::fs::remove_file(fixture.0.join("areas/00_00/cells.bin")).unwrap();
        assert!(world.area(0, 0).is_err());
    }

    #[test]
    fn borrowed_area_is_cached_and_safe_for_concurrent_readers() {
        let fixture = Fixture::new();
        fixture.write_area(0, 0);
        let world = World::load(&fixture.0).unwrap();
        std::thread::scope(|scope| {
            let a = scope.spawn(|| world.area(0, 0).unwrap());
            let b = scope.spawn(|| world.area(0, 0).unwrap());
            assert!(std::ptr::eq(a.join().unwrap(), b.join().unwrap()));
        });
        std::fs::remove_file(fixture.0.join("areas/00_00/cells.bin")).unwrap();
        assert_eq!(
            world
                .area(0, 0)
                .unwrap()
                .cell(511, 511)
                .unwrap()
                .height
                .raw(),
            1234
        );
        assert!(world
            .block(0, 0, 0, 0)
            .unwrap_err()
            .to_string()
            .contains("00_00.tiles.zst"));
    }

    #[test]
    fn malformed_manifest_dimensions_fail_before_allocating_caches() {
        let fixture = Fixture::new();
        let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
        manifest.areas_wide = i32::MAX;
        arda_core::write_manifest(&fixture.0, &manifest).unwrap();
        assert!(matches!(
            World::load(&fixture.0),
            Err(LoadError::ManifestUnreadable { .. })
        ));
    }

    fn saved_context(objects: &AreaObjects) -> (Fixture, World) {
        let fixture = Fixture::new();
        fixture.write_area(0, 0);
        std::fs::write(
            fixture.0.join("areas/00_00/objects.bin"),
            arda_core::encode_objects(objects).unwrap(),
        )
        .unwrap();
        let world = World::load(&fixture.0).unwrap();
        (fixture, world)
    }

    fn spill_objects(
        from: arda_core::GlobalCell,
        to: Option<arda_core::GlobalCell>,
    ) -> AreaObjects {
        use arda_core::hydrology::{
            BasinId, GlobalLake, Litres, ReceivingAccount, SpillConnection,
        };
        let mut objects = AreaObjects::empty();
        objects.global.lakes.push(GlobalLake {
            basin: BasinId(1),
            surface: HeightMm::new(1),
            deepest_bed: HeightMm::new(0),
            submerged_cells: 1,
            outlet: Some(SpillConnection {
                from,
                to,
                sill: HeightMm::new(1),
                receiving: if to.is_some() {
                    ReceivingAccount::Sea
                } else {
                    ReceivingAccount::DomainExport
                },
            }),
            annual_outflow: Litres(1),
            mean_outflow: arda_core::DischargeMilli::new(0),
        });
        objects
    }

    #[test]
    fn saved_spills_use_the_actual_requested_exported_union_rim() {
        use arda_core::GlobalCell;
        for objects in [
            spill_objects(GlobalCell { x: 20, y: 20 }, None),
            spill_objects(
                GlobalCell { x: 1023, y: 20 },
                Some(GlobalCell { x: 1024, y: 20 }),
            ),
            spill_objects(GlobalCell { x: 20, y: 2048 }, None),
        ] {
            let (fixture, world) = saved_context(&objects);
            let error = world.read_area(0, 0).unwrap_err();
            assert!(matches!(
                error,
                LoadError::Corrupt {
                    source: arda_core::FormatError::Hydrology { path, .. }
                } if Path::new(&path) == fixture.0.join("areas").join("00_00").join("objects.bin")
            ));
        }
        // MICRO's exported overshoot is part of the physical domain.
        let (_fixture, world) =
            saved_context(&spill_objects(GlobalCell { x: 1023, y: 2047 }, None));
        assert!(world.read_area(0, 0).is_ok());
        // A 64 km request has a private fringe beyond its single 512-cell export.
        let (fixture, _) = saved_context(&spill_objects(GlobalCell { x: 639, y: 300 }, None));
        let mut manifest = arda_core::read_manifest(&fixture.0).unwrap();
        manifest.config = GenerateConfig::new(
            SizeKm {
                width: 64,
                height: 64,
            },
            manifest.config.latitude_band(),
            manifest.config.mean_density_per_km2(),
        )
        .unwrap();
        manifest.areas_wide = 1;
        manifest.areas_high = 1;
        arda_core::write_manifest(&fixture.0, &manifest).unwrap();
        let world = World::load(&fixture.0).unwrap();
        assert!(world.read_area(0, 0).is_ok());
        assert!(!fixture.0.join("hydrology").exists());
    }

    #[test]
    fn copied_coordinates_and_receiving_identities_cannot_leave_the_manifest_domain() {
        use arda_core::hydrology::{
            AnnualCatchment, CatchmentId, ChannelEdge, GlobalReach, Litres, ReachId,
            ReceivingAccount, SharedCrossing,
        };
        use arda_core::{DischargeMilli, GlobalCell};
        let at = |x, y| GlobalCell { x, y };
        let owner = AnnualCatchment {
            catchment: CatchmentId(1),
            terminal: at(0, 0),
            contributing_cells: 1,
            basin: None,
            representative_lake: None,
            potential_spill: None,
            receiving: ReceivingAccount::Sea,
        };
        let reach = GlobalReach {
            id: ReachId::from_step(at(20, 20), at(21, 20)).unwrap(),
            from: at(20, 20),
            to: at(21, 20),
            receiving: ReceivingAccount::Sea,
            catchment: owner.catchment,
            drainage_cells: 1,
            annual_volume: Litres(40 * 31_536_000),
            mean_discharge: DischargeMilli::new(40),
        };
        let mut cases = Vec::new();
        let mut objects = AreaObjects::empty();
        objects.global.catchments.push(AnnualCatchment {
            terminal: at(1024, 0),
            ..owner
        });
        cases.push(objects);
        let mut objects = AreaObjects::empty();
        let bad_spill = spill_objects(at(20, 20), None).global.lakes[0]
            .outlet
            .unwrap();
        objects.global.catchments.push(AnnualCatchment {
            basin: Some(arda_core::hydrology::BasinId(1)),
            potential_spill: Some(bad_spill),
            receiving: ReceivingAccount::DomainExport,
            ..owner.clone()
        });
        cases.push(objects);
        for bad in [
            GlobalReach {
                id: ReachId::from_step(at(1023, 20), at(1024, 20)).unwrap(),
                from: at(1023, 20),
                to: at(1024, 20),
                ..reach.clone()
            },
            GlobalReach {
                receiving: ReceivingAccount::Reach(
                    ReachId::from_step(at(1024, 0), at(1025, 0)).unwrap(),
                ),
                ..reach.clone()
            },
            GlobalReach {
                id: ReachId::point(at(20, 20)).unwrap(),
                from: at(20, 20),
                to: at(20, 20),
                receiving: ReceivingAccount::DomainExport,
                ..reach.clone()
            },
        ] {
            let mut objects = AreaObjects::empty();
            objects.global.catchments.push(owner.clone());
            objects.global.reaches.push(bad);
            cases.push(objects);
        }
        let mut objects = AreaObjects::empty();
        objects.global.catchments.push(owner.clone());
        objects.global.reaches.push(reach.clone());
        objects.global.crossings.push(SharedCrossing {
            id: arda_core::hydrology::CrossingId {
                low: at(1023, 20),
                high: at(1024, 20),
            },
            from: at(1023, 20),
            to: at(1024, 20),
            reach: reach.id,
            catchment: owner.catchment,
            drainage_cells: 1,
            annual_volume: reach.annual_volume,
            mean_discharge: reach.mean_discharge,
            receiving: ReceivingAccount::Sea,
        });
        cases.push(objects);
        let mut objects = AreaObjects::empty();
        objects.channel_edges.push(ChannelEdge {
            from: at(1023, 1),
            to: at(1024, 1),
            from_width_dm: 8,
            to_width_dm: 8,
            discharge: DischargeMilli::new(40),
        });
        cases.push(objects);
        for objects in cases {
            let (_fixture, world) = saved_context(&objects);
            assert!(world.read_area(0, 0).is_err());
        }
    }

    #[test]
    fn oversized_requested_layers_are_rejected_before_read_allocation() {
        use std::io::Write;

        for name in ["cells.bin", "objects.bin"] {
            let fixture = Fixture::new();
            fixture.write_area(0, 0);
            let dir = fixture.0.join("areas").join("00_00");
            let layer_bytes =
                |name| usize::try_from(std::fs::metadata(dir.join(name)).unwrap().len()).unwrap();
            let cell_bytes = layer_bytes("cells.bin");
            let object_bytes = layer_bytes("objects.bin");
            let world = World::load(&fixture.0).unwrap();
            assert!(world.read_area(0, 0).is_ok());
            assert!(world
                .read_area_with_byte_limits(0, 0, cell_bytes, object_bytes)
                .is_ok());

            // Small explicit caps exercise the same admission path on filesystems
            // where extending a file allocates every byte rather than sparse space.
            let path = dir.join(name);
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap()
                .write_all(&[0])
                .unwrap();
            let error = world
                .read_area_with_byte_limits(0, 0, cell_bytes, object_bytes)
                .unwrap_err();
            assert!(matches!(
                error,
                LoadError::Corrupt {
                    source: arda_core::FormatError::Hydrology {
                        path: error_path,
                        source: arda_core::formats::hydrology::HydrologyFormatError::Limit(
                            "saved layer byte limit"
                        ),
                    }
                } if Path::new(&error_path) == path
            ));
        }
    }
}
