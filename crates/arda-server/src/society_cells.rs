//! The world's `society/` rasters as cell-contract fields (adapter A11,
//! logic/16 §api-cell-society): `road` from `roads.bin`, `built_by` and
//! `land_use` from `landuse.bin`, `realm_id` from `realms.bin`.

use crate::error::{ServerError, ServerResult};
use std::path::Path;

/// Bytes held per cell: land-use code, owner, realm and road code.
pub const BYTES_PER_CELL: usize = 1 + 4 + 2 + 1;

/// The society values of one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SocietyCell {
    /// `arda_ids::LandUse` code.
    pub land_use: u8,
    /// Owning settlement id, 0 for none.
    pub owner: u32,
    /// Realm id, 0 for none.
    pub realm: u16,
    /// `arda_ids::RoadClass` code; `None` when the world's society has no
    /// `roads.bin` (written before it existed), so the stored cell road stands.
    pub road: Option<u8>,
}

/// The decoded society rasters, one value per global cell, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocietyCells {
    width: usize,
    height: usize,
    land_use: Vec<u8>,
    owner: Vec<u32>,
    realm: Vec<u16>,
    road: Option<Vec<u8>>,
}

fn load_err(what: &str, e: &impl std::fmt::Display) -> ServerError {
    ServerError::Internal(format!("society {what}: {e}"))
}

impl SocietyCells {
    /// Reads `<society>/landuse.bin`, `realms.bin` and, when present,
    /// `roads.bin`; `Ok(None)` when the world has no `landuse.bin`.
    ///
    /// # Errors
    /// [`ServerError::Internal`] when a raster is unreadable or its size is
    /// not the world's `cells_wide × cells_high` (a stale `society/`).
    pub fn open(society: &Path, cells_wide: u32, cells_high: u32) -> ServerResult<Option<Self>> {
        let landuse = society.join("landuse.bin");
        if !landuse.exists() {
            return Ok(None);
        }
        let want = (cells_wide as usize, cells_high as usize);
        let fits = |what: &str, w: usize, h: usize| {
            if (w, h) == want {
                Ok(())
            } else {
                Err(ServerError::Internal(format!(
                    "society {what} is {w} × {h} cells, the world {} × {}",
                    want.0, want.1
                )))
            }
        };
        let (w, h, land_use, owner) =
            arda_settle::output::read_landuse(&landuse).map_err(|e| load_err("landuse.bin", &e))?;
        fits("landuse.bin", w, h)?;
        let (rw, rh, realm) = arda_settle::output::read_realm_map(&society.join("realms.bin"))
            .map_err(|e| load_err("realms.bin", &e))?;
        fits("realms.bin", rw, rh)?;
        let roads = society.join("roads.bin");
        let road = if roads.exists() {
            let (dw, dh, codes) = arda_settle::output::read_road_map(&roads)
                .map_err(|e| load_err("roads.bin", &e))?;
            fits("roads.bin", dw, dh)?;
            Some(codes)
        } else {
            None
        };
        Ok(Some(Self {
            width: w,
            height: h,
            land_use,
            owner,
            realm,
            road,
        }))
    }

    /// Resident bytes.
    #[must_use]
    pub const fn bytes(&self) -> usize {
        self.width * self.height * BYTES_PER_CELL
    }

    /// The values of global cell `(gx, gy)`; default (none) outside.
    #[must_use]
    pub fn at(&self, gx: u32, gy: u32) -> SocietyCell {
        let (x, y) = (gx as usize, gy as usize);
        if x >= self.width || y >= self.height {
            return SocietyCell::default();
        }
        let i = y * self.width + x;
        SocietyCell {
            land_use: self.land_use.get(i).copied().unwrap_or(0),
            owner: self.owner.get(i).copied().unwrap_or(0),
            realm: self.realm.get(i).copied().unwrap_or(0),
            road: self.road.as_ref().map(|r| r.get(i).copied().unwrap_or(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("arda-server-society-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, w: usize, h: usize, roads: bool) {
        let n = w * h;
        let codes: Vec<u8> = (0..n).map(|i| u8::try_from(i % 11).unwrap()).collect();
        let owner: Vec<u32> = (0..n).map(|i| u32::try_from(i).unwrap()).collect();
        let realm: Vec<u16> = (0..n).map(|i| u16::try_from(i % 3).unwrap()).collect();
        arda_settle::output::write_landuse(&dir.join("landuse.bin"), w, h, &codes, &owner).unwrap();
        arda_settle::output::write_realm_map(&dir.join("realms.bin"), w, h, &realm).unwrap();
        if roads {
            let r: Vec<u8> = (0..n).map(|i| u8::try_from(i % 5).unwrap()).collect();
            arda_settle::output::write_road_map(&dir.join("roads.bin"), w, h, &r).unwrap();
        }
    }

    #[test]
    fn reads_every_raster_by_global_cell() {
        let dir = scratch("read");
        write(&dir, 4, 3, true);
        let s = SocietyCells::open(&dir, 4, 3).unwrap().unwrap();
        // Cell (1, 2) is index 9.
        let c = s.at(1, 2);
        assert_eq!(
            c,
            SocietyCell {
                land_use: 9,
                owner: 9,
                realm: 0,
                road: Some(4)
            }
        );
        assert_eq!(s.at(4, 0), SocietyCell::default());
        assert_eq!(s.bytes(), 12 * BYTES_PER_CELL);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_society_without_roads_bin_keeps_the_stored_road() {
        let dir = scratch("noroads");
        write(&dir, 2, 2, false);
        let s = SocietyCells::open(&dir, 2, 2).unwrap().unwrap();
        assert_eq!(s.at(1, 1).road, None);
        assert_eq!(s.at(1, 1).owner, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_society_is_refused_and_a_missing_one_is_none() {
        let dir = scratch("stale");
        assert!(SocietyCells::open(&dir, 2, 2).unwrap().is_none());
        write(&dir, 2, 2, true);
        let err = SocietyCells::open(&dir, 3, 2).unwrap_err();
        assert!(err.to_string().contains("landuse.bin"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
