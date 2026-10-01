//! Opt-in fine source selection and recipe-4 valley formation before publication.

use super::{
    climate, extract_rivers, fine_formation, fine_input, fine_source, fine_valleys, fine_world,
    generate_world_from_fine_terrain, generation_limits, hydrology, river_gate, GenError,
    HydrologyLimits, WorldOutput, CONTINENT_ATTEMPTS, LAND_FRACTION_GATE,
};
use crate::continent::generate_continent_attempt_fine;
use arda_core::{FineTerrainDescriptor, GenerateConfig, Manifest, TerrainFileReader};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Separate source and world resource ceilings. The source's fixed five
/// attempts have bounded dimensions/file bytes, but their spectral work is not
/// charged to the world's hydrology-operation counter.
#[derive(Debug, Clone, Copy)]
pub struct FineDeliveryLimits {
    /// Per-attempt source RAM and finalized file ceiling.
    pub source: fine_source::FineSourceLimits,
    /// World admission after reserving raw and formed source file staging.
    pub world: HydrologyLimits,
}

impl Default for FineDeliveryLimits {
    fn default() -> Self {
        // Fine recipe 4 has substantially more resolved closed terminals than
        // the legacy source. This capacity exceeds the 194,453 counted in the
        // seed-42 500 x 1000 km routing authority, with about 29% headroom.
        let world = HydrologyLimits {
            max_closed_leaves: 250_000,
            ..HydrologyLimits::default()
        };
        Self {
            source: fine_source::FineSourceLimits {
                max_ram_bytes: 16 * 1024 * 1024 * 1024,
                max_file_bytes: 4 * 1024 * 1024 * 1024,
            },
            world,
        }
    }
}

struct SourceStage {
    path: PathBuf,
    removed: bool,
}

impl SourceStage {
    fn create(out: &Path) -> Result<Self, GenError> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let parent = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|source| io(parent, source))?;
        for _ in 0..32 {
            let path = parent.join(format!(
                ".arda-fine-source-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        removed: false,
                    })
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(source) => return Err(io(&path, source)),
            }
        }
        Err(GenError::Validation {
            check: "no unused private fine-source staging name".into(),
        })
    }

    fn source(&self) -> PathBuf {
        self.path.join("candidate.terrain")
    }

    fn valleys(&self) -> PathBuf {
        self.path.join("candidate-valleys.terrain")
    }

    fn remove(mut self) -> Result<(), GenError> {
        fs::remove_dir_all(&self.path).map_err(|source| io(&self.path, source))?;
        self.removed = true;
        Ok(())
    }
}

impl Drop for SourceStage {
    fn drop(&mut self) {
        if !self.removed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn io(path: &Path, source: std::io::Error) -> GenError {
    GenError::Write {
        path: path.display().to_string(),
        source,
    }
}

fn refuse_occupied(out: &Path) -> Result<(), GenError> {
    if !out.exists() {
        return Ok(());
    }
    let mut entries = fs::read_dir(out).map_err(|source| io(out, source))?;
    if entries
        .next()
        .transpose()
        .map_err(|source| io(out, source))?
        .is_some()
    {
        return Err(GenError::OutputNotEmpty {
            dir: out.display().to_string(),
        });
    }
    Ok(())
}

/// Canonical fine terrain recipe produced by [`generate_world_with_fine_recipe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FineRecipe {
    /// Recipe 4: spectral source plus carved bent valleys (exact replay).
    Valleys,
    /// Recipe 6, the default: multi-resolution stream-power formation with
    /// margins, coasts and water forms (logic/02 §fine-formation).
    Formation,
    /// Recipe 5: stream-power formation exactly as v0.1 shipped it.
    FormationV5,
}

impl FineRecipe {
    /// The manifest `recipe_version` this recipe records.
    #[must_use]
    pub const fn version(self) -> u16 {
        match self {
            Self::Valleys => 4,
            Self::FormationV5 => 5,
            Self::Formation => 6,
        }
    }

    /// The recipe that generates `recipe_version`, if this build has one
    /// (recipes 4, 5 and 6).
    #[must_use]
    pub const fn from_version(recipe_version: u16) -> Option<Self> {
        match recipe_version {
            4 => Some(Self::Valleys),
            5 => Some(Self::FormationV5),
            6 => Some(Self::Formation),
            _ => None,
        }
    }

    /// The formation rule set, for the stream-power recipes.
    const fn formation(self) -> Option<crate::formation::Recipe> {
        match self {
            Self::Valleys => None,
            Self::FormationV5 => Some(crate::formation::Recipe::V5),
            Self::Formation => Some(crate::formation::Recipe::V6),
        }
    }
}

/// Generate one default-recipe (6) world; see
/// [`generate_world_with_fine_recipe`].
///
/// # Errors
/// As [`generate_world_with_fine_recipe`].
pub fn generate_world_with_fine_source(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: FineDeliveryLimits,
) -> Result<Manifest, GenError> {
    generate_world_with_fine_recipe(seed, config, out, limits, FineRecipe::Formation)
}

/// Generate one world from the first of five deterministic, validated fine
/// sources of the requested recipe. Rejected candidates
/// never touch the final world directory.
/// No failed candidate falls back to legacy terrain.
///
/// The staged source is private and deleted after success or on ordinary error;
/// a process crash may leave its named staging directory for manual removal.
/// World failure retains the existing partial-output clean-rerun contract.
///
/// # Errors
/// Returns source admission/generation, continent gate, world admission, or
/// publication errors without silently changing the seed, recipe, or limits.
pub fn generate_world_with_fine_recipe(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: FineDeliveryLimits,
    recipe: FineRecipe,
) -> Result<Manifest, GenError> {
    let plan = match recipe.formation() {
        None => fine_source::admit(config, limits.source)?,
        Some(_) => {
            let plan = fine_source::admit(
                config,
                fine_source::FineSourceLimits {
                    max_ram_bytes: u128::MAX,
                    max_file_bytes: limits.source.max_file_bytes,
                },
            )?;
            crate::formation::plan(
                plan.fine_width,
                plan.fine_height,
                limits.source.max_ram_bytes,
            )?;
            plan
        }
    };
    let source_bytes = u64::try_from(plan.file_bytes).map_err(|_| GenError::ResourceEnvelope {
        resource: "staged fine file",
        required: plan.file_bytes,
        limit: u128::from(u64::MAX),
    })?;
    let staged_peak = source_bytes
        .checked_mul(2)
        .ok_or(GenError::ResourceEnvelope {
            resource: "fine source staging and world copy",
            required: u128::from(source_bytes) * 2,
            limit: u128::from(limits.world.combined_scratch_bytes),
        })?;
    if staged_peak > limits.world.combined_scratch_bytes {
        return Err(GenError::ResourceEnvelope {
            resource: "fine source staging and world copy",
            required: u128::from(staged_peak),
            limit: u128::from(limits.world.combined_scratch_bytes),
        });
    }
    let mut world_limits = limits.world;
    // Peak staging is raw+valley during formation, then retained valley+
    // copied world terrain during the solve. Conservatively retain both file
    // charges alongside all admitted world scratch in either phase.
    world_limits.combined_scratch_bytes = world_limits
        .combined_scratch_bytes
        .checked_sub(staged_peak)
        .ok_or(GenError::ResourceEnvelope {
            resource: "world scratch plus staged and copied fine terrain",
            required: u128::from(staged_peak),
            limit: u128::from(limits.world.combined_scratch_bytes),
        })?;
    let fine_admission = fine_world::admit_bytes(source_bytes, config, world_limits)?;
    let world_admission = generation_limits::admit(
        config,
        &WorldOutput::scratch_path(out),
        fine_admission.remaining,
    )?;
    refuse_occupied(out)?;

    let stage = SourceStage::create(out)?;
    let path = stage.source();
    let valley_path = stage.valleys();
    let mut last_check = String::new();
    for attempt in 0..CONTINENT_ATTEMPTS {
        let macro_grid = match recipe.formation() {
            Some(_) => crate::continent::generate_continent_attempt_formed(seed, config, attempt),
            None => generate_continent_attempt_fine(seed, config, attempt),
        };
        let mut water = None;
        let recipe_version = match recipe.formation() {
            Some(formation) => {
                water = fine_formation::generate(
                    seed,
                    attempt,
                    config,
                    &macro_grid,
                    &path,
                    limits.source,
                    formation,
                )?;
                drop(macro_grid);
                formation.version()
            }
            None => {
                let receipt = fine_source::generate(
                    seed,
                    attempt,
                    config,
                    &macro_grid,
                    &path,
                    limits.source,
                )?;
                drop(macro_grid);
                let mut reader = TerrainFileReader::open(
                    File::open(&path).map_err(|source| io(&path, source))?,
                    1 << 20,
                )
                .map_err(fine_input::FineInputError::from)?;
                let initial = fine_input::continent(
                    &mut reader,
                    config,
                    world_admission.reservations.continent_preparation_bytes,
                )?;
                drop(reader);
                let initial_climate = climate(&initial, config.latitude_band());
                let initial_hydro = hydrology(&initial, &initial_climate);
                let _valleys =
                    fine_valleys::carve(&path, &valley_path, &initial, &initial_hydro, seed)?;
                drop(initial_hydro);
                drop(initial_climate);
                drop(initial);
                // Both candidates are private. Select the finalized/checksummed
                // valley field before any climate, routing or published height.
                fs::remove_file(&path).map_err(|source| io(&path, source))?;
                fs::rename(&valley_path, &path).map_err(|source| io(&valley_path, source))?;
                // Recipe 3 is the unformed macro/spectral source. The
                // completed valley transform is recipe 4 authority.
                receipt.recipe_version + 1
            }
        };
        let mut reader = TerrainFileReader::open(
            File::open(&path).map_err(|source| io(&path, source))?,
            1 << 20,
        )
        .map_err(fine_input::FineInputError::from)?;
        let candidate = fine_input::continent(
            &mut reader,
            config,
            world_admission.reservations.continent_preparation_bytes,
        )?;
        drop(reader);
        let land = candidate.land_fraction_permille();
        if !LAND_FRACTION_GATE.contains(&land) {
            last_check = format!(
                "land fraction {land} per mille outside {}..={}",
                LAND_FRACTION_GATE.start(),
                LAND_FRACTION_GATE.end()
            );
        } else {
            let clim = if recipe_version >= 5 {
                crate::continent::climate::climate_smoothed_rain(&candidate, config.latitude_band())
            } else {
                climate(&candidate, config.latitude_band())
            };
            let hydro = hydrology(&candidate, &clim);
            let rivers = extract_rivers(&candidate, &hydro);
            match river_gate(&rivers) {
                Ok(()) => {
                    let descriptor = FineTerrainDescriptor {
                        recipe_version,
                        attempt,
                    };
                    // The preflight's full continent/climate/flow graph must not
                    // overlap the admitted world-generation peak.
                    drop(rivers);
                    drop(hydro);
                    drop(clim);
                    drop(candidate);
                    let manifest = match &water {
                        Some(w) => super::generate_world_from_formed_terrain(
                            seed,
                            descriptor,
                            config,
                            &path,
                            out,
                            world_limits,
                            w,
                        )?,
                        None => generate_world_from_fine_terrain(
                            seed,
                            descriptor,
                            config,
                            &path,
                            out,
                            world_limits,
                        )?,
                    };
                    stage.remove()?;
                    return Ok(manifest);
                }
                Err(check) => last_check = check,
            }
        }
        fs::remove_file(&path).map_err(|source| io(&path, source))?;
    }
    stage.remove()?;
    Err(GenError::Validation {
        check: format!("{last_check} after {CONTINENT_ATTEMPTS} fine-source candidate(s)"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{LatitudeBand, SizeKm};

    #[test]
    fn full_fine_world_admits_observed_closed_leaves_with_complete_resource_envelope() {
        let config =
            GenerateConfig::new(SizeKm::new(500, 1000), LatitudeBand::new(35, 55), 15).unwrap();
        let limits = FineDeliveryLimits::default();
        let observed = 194_453;
        assert!(HydrologyLimits::default().max_closed_leaves < observed);
        assert!(limits.world.max_closed_leaves >= observed);
        let source = fine_source::admit(config, limits.source).unwrap();
        let source_bytes = u64::try_from(source.file_bytes).unwrap();
        let staged_peak = source_bytes * 2;
        let mut world_limits = limits.world;
        world_limits.combined_scratch_bytes -= staged_peak;
        let fine = fine_world::admit_bytes(source_bytes, config, world_limits).unwrap();
        let admission = generation_limits::admit(
            config,
            Path::new("/tmp/arda-fine-500x1000-admission/.arda-hydrology-scratch"),
            fine.remaining,
        )
        .unwrap();
        let r = admission.reservations;
        assert_eq!(
            admission.shared.mst.max_leaves,
            limits.world.max_closed_leaves
        );
        assert!(r.ram_bytes <= fine.remaining.ram_bytes);
        assert!(r.spatial_scratch_bytes <= fine.remaining.spatial_scratch_bytes);
        assert!(r.scratch_bytes + staged_peak <= limits.world.combined_scratch_bytes);
        assert!(r.io_bytes <= fine.remaining.global_io_bytes);
        assert!(r.io_operations <= fine.remaining.global_io_operations);
        assert!(r.logical_operations <= fine.remaining.global_event_operations);
        eprintln!(
            "fine500x1000: source_bytes={source_bytes} leaves={} mst_scratch={} ram={} scratch_with_source={} io_bytes={} io_calls={} work={}",
            admission.shared.mst.max_leaves,
            admission.shared.mst.scratch_bytes,
            r.ram_bytes,
            r.scratch_bytes + staged_peak,
            r.io_bytes,
            r.io_operations,
            r.logical_operations,
        );
    }

    #[test]
    fn source_admission_refuses_before_touching_world_or_stage() {
        let parent = std::env::temp_dir().join(format!("arda-fine-admit-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let out = parent.join("world");
        let mut limits = FineDeliveryLimits::default();
        limits.source.max_ram_bytes = 0;
        assert!(generate_world_with_fine_source(42, GenerateConfig::MICRO, &out, limits).is_err());
        assert!(!out.exists());
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
        fs::remove_dir(&parent).unwrap();
    }

    #[test]
    fn world_admission_and_occupied_output_refuse_before_staging() {
        let parent = std::env::temp_dir().join(format!("arda-fine-refuse-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let out = parent.join("world");
        let mut limits = FineDeliveryLimits::default();
        limits.world.ram_bytes = 0;
        assert!(generate_world_with_fine_source(42, GenerateConfig::MICRO, &out, limits).is_err());
        assert!(!out.exists());
        fs::create_dir(&out).unwrap();
        fs::write(out.join("keep"), b"user data").unwrap();
        assert!(matches!(
            generate_world_with_fine_source(
                42,
                GenerateConfig::MICRO,
                &out,
                FineDeliveryLimits::default(),
            ),
            Err(GenError::OutputNotEmpty { .. })
        ));
        assert_eq!(fs::read(out.join("keep")).unwrap(), b"user data");
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 1);
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn private_stage_cleanup_does_not_remove_unrelated_sibling() {
        let parent = std::env::temp_dir().join(format!("arda-fine-stage-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let keep = parent.join("keep");
        fs::write(&keep, b"user data").unwrap();
        let stage = SourceStage::create(&parent.join("world")).unwrap();
        let staged = stage.source();
        fs::write(&staged, b"candidate").unwrap();
        stage.remove().unwrap();
        assert!(!staged.exists());
        assert_eq!(fs::read(&keep).unwrap(), b"user data");
        fs::remove_file(keep).unwrap();
        fs::remove_dir(parent).unwrap();
    }
}
