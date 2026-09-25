//! Opt-in recipe-2 source selection before final world publication.

use super::{
    climate, extract_rivers, fine_input, fine_source, fine_world, generate_continent_attempt,
    generate_world_from_fine_terrain, generation_limits, hydrology, river_gate, GenError,
    HydrologyLimits, WorldOutput, CONTINENT_ATTEMPTS, LAND_FRACTION_GATE,
};
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
    /// World admission after reserving one simultaneously staged source file.
    pub world: HydrologyLimits,
}

impl Default for FineDeliveryLimits {
    fn default() -> Self {
        Self {
            source: fine_source::FineSourceLimits {
                max_ram_bytes: 16 * 1024 * 1024 * 1024,
                max_file_bytes: 4 * 1024 * 1024 * 1024,
            },
            world: HydrologyLimits::default(),
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

/// Generate one world from the first of five deterministic, validated recipe-2
/// fine sources. Rejected candidates never touch the final world directory.
/// No failed candidate falls back to legacy terrain.
///
/// The staged source is private and deleted after success or on ordinary error;
/// a process crash may leave its named staging directory for manual removal.
/// World failure retains the existing partial-output clean-rerun contract.
///
/// # Errors
/// Returns source admission/generation, continent gate, world admission, or
/// publication errors without silently changing the seed, recipe, or limits.
pub fn generate_world_with_fine_source(
    seed: u64,
    config: GenerateConfig,
    out: &Path,
    limits: FineDeliveryLimits,
) -> Result<Manifest, GenError> {
    let plan = fine_source::admit(config, limits.source)?;
    let source_bytes = u64::try_from(plan.file_bytes).map_err(|_| GenError::ResourceEnvelope {
        resource: "staged fine file",
        required: plan.file_bytes,
        limit: u128::from(u64::MAX),
    })?;
    let mut world_limits = limits.world;
    world_limits.combined_scratch_bytes = world_limits
        .combined_scratch_bytes
        .checked_sub(source_bytes)
        .ok_or(GenError::ResourceEnvelope {
            resource: "world scratch plus staged fine source",
            required: u128::from(source_bytes),
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
    let mut last_check = String::new();
    for attempt in 0..CONTINENT_ATTEMPTS {
        let macro_grid = generate_continent_attempt(seed, config, attempt);
        let receipt =
            fine_source::generate(seed, attempt, config, &macro_grid, &path, limits.source)?;
        drop(macro_grid);
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
            let clim = climate(&candidate, config.latitude_band());
            let hydro = hydrology(&candidate, &clim);
            let rivers = extract_rivers(&candidate, &hydro);
            match river_gate(&rivers) {
                Ok(()) => {
                    let descriptor = FineTerrainDescriptor {
                        recipe_version: receipt.recipe_version,
                        attempt: receipt.attempt,
                    };
                    // The preflight's full continent/climate/flow graph must not
                    // overlap the admitted world-generation peak.
                    drop(rivers);
                    drop(hydro);
                    drop(clim);
                    drop(candidate);
                    let manifest = generate_world_from_fine_terrain(
                        seed,
                        descriptor,
                        config,
                        &path,
                        out,
                        world_limits,
                    )?;
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
