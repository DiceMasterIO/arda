//! Input guards for the overlay (review round 2, #12): the overlay reads
//! `society/` files that may be stale, truncated or hostile, so sizes and
//! positions are checked before any indexing or metre → pixel arithmetic,
//! and the scratch directory is created exclusively.

use super::Society;
use crate::error::SettleError;
use std::path::{Path, PathBuf};

fn bad(reason: &'static str) -> SettleError {
    SettleError::Format {
        path: "society overlay".into(),
        reason,
    }
}

/// Refuses inputs `draw` cannot index or scale: an empty grid, rasters of
/// another size, and positions off the world (which overflow the metre →
/// pixel arithmetic or draw lines billions of pixels long).
///
/// # Errors
/// [`SettleError::Format`].
pub fn check(soc: &Society) -> Result<(), SettleError> {
    let (gw, gh) = soc.size;
    let n = gw.checked_mul(gh).ok_or(bad("grid size"))?;
    if n == 0 || soc.codes.len() != n || soc.realm_map.len() != n {
        return Err(bad("rasters are empty or of different sizes"));
    }
    let ui = |v: usize| i64::try_from(v).unwrap_or(i64::MAX);
    let (xmax, ymax) = (ui(gw).saturating_mul(100), ui(gh).saturating_mul(100));
    let on = |m: [i64; 2]| (0..=xmax).contains(&m[0]) && (0..=ymax).contains(&m[1]);
    let roads = &soc.roads;
    let mut points = soc
        .settlements
        .settlements
        .iter()
        .map(|s| [s.x_m, s.y_m])
        .chain(
            roads
                .roads
                .iter()
                .flat_map(|r| r.segments.iter().flatten().copied()),
        )
        .chain(roads.crossings.iter().map(|x| [x.x_m, x.y_m]))
        .chain(roads.passes.iter().map(|p| [p.x_m, p.y_m]));
    if points.any(|p| !on(p)) {
        return Err(bad("a position lies off the world"));
    }
    Ok(())
}

/// A fresh private scratch directory under `base`. `create_dir` refuses a
/// path that already exists (a directory or symlink planted at the
/// predictable name), and the next name is tried.
///
/// # Errors
/// [`SettleError::Io`] when every name is taken.
pub fn scratch_dir(base: &Path) -> Result<PathBuf, SettleError> {
    let pid = std::process::id();
    let mut last = None;
    for k in 0..64_u32 {
        let dir = base.join(format!("arda-settle-render-{pid}-{k}"));
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) => last = Some((dir, e)),
        }
    }
    let (dir, e) = last.unwrap_or_else(|| {
        (
            base.to_path_buf(),
            std::io::Error::other("no scratch directory"),
        )
    });
    Err(SettleError::io(&dir, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::{NamesFile, RealmsFile, RoadsFile, SettlementsFile};
    use crate::roads::{Road, RoadClass, Terrain};

    fn society(segment: Vec<[i64; 2]>, size: (usize, usize), realm_cells: usize) -> Society {
        Society {
            settlements: SettlementsFile {
                format_version: 1,
                seed: 1,
                width_cells: 10,
                height_cells: 10,
                settlements: vec![],
            },
            roads: RoadsFile {
                format_version: 1,
                roads: vec![Road {
                    id: 1,
                    class: RoadClass::Track,
                    from: 1,
                    to: None,
                    to_edge: None,
                    length_m: 0,
                    straight_m: 0,
                    new_m: 0,
                    relief_m: 0,
                    terrain: Terrain::Open,
                    segments: vec![segment],
                }],
                crossings: vec![],
                passes: vec![],
            },
            realms: RealmsFile {
                format_version: 1,
                realms: vec![],
            },
            names: NamesFile {
                format_version: 1,
                rivers: vec![],
                mountains: vec![],
                regions: vec![],
            },
            codes: vec![0; size.0 * size.1],
            realm_map: vec![0; realm_cells],
            size,
        }
    }

    #[test]
    fn mismatched_empty_or_off_world_inputs_are_errors_not_panics() {
        let on = vec![[50, 50], [950, 50]];
        assert!(check(&society(on.clone(), (10, 10), 100)).is_ok());
        // A stale realms.bin smaller than the land use indexed out of bounds.
        assert!(check(&society(on.clone(), (10, 10), 4)).is_err());
        // A zero-width grid divided by zero.
        assert!(check(&society(on, (0, 10), 0)).is_err());
        // A road point at 10^15 m overflowed i64 and drew a 10^12-step line.
        let far = vec![[0, 0], [1_000_000_000_000_000, 0]];
        assert!(check(&society(far, (10, 10), 100)).is_err());
    }

    #[test]
    fn scratch_dirs_never_reuse_a_planted_path() {
        let base = std::env::temp_dir().join(format!("arda-settle-scratch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let planted = base.join(format!("arda-settle-render-{}-0", std::process::id()));
        std::fs::create_dir(&planted).unwrap();
        let dir = scratch_dir(&base).unwrap();
        assert_ne!(dir, planted);
        let _ = std::fs::remove_dir_all(&base);
    }
}
