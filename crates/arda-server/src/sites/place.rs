//! Where a world's sites lie (logic/18 §site-place): dungeons beside
//! settlements, caves at the foot of steep ground. A pure function of the
//! world seed, the cells and the settlements.

use super::SiteId;
use arda_dungeon::Kind;
use arda_ids::hash::subseed;
use arda_ids::SettlementId;
use std::collections::BTreeSet;

/// Side of a cave tile, cells (3.2 km). Fixed: density moves
/// [`CAVE_ODDS`], never the grid, so a denser world keeps its caves.
pub const TILE: u32 = 32;
/// A tile's steepest cell must slope at least this.
pub const STEEP_MILLI_DEG: u16 = 15_000;
/// A cave's entrance cell must slope less than this.
pub const WALKABLE_MILLI_DEG: u16 = 15_000;
/// Share of qualifying tiles that keep their cave, per mille.
pub const CAVE_ODDS: u64 = 300;
/// Share of villages that get a dungeon, per mille.
pub const VILLAGE_ODDS: u64 = 100;
/// A dungeon's entrance lies this many cells from its settlement.
pub const RING: std::ops::RangeInclusive<u32> = 2..=16;
/// A town or city this near (cells) makes a site's base tier 1.
pub const NEAR_CELLS: i64 = 150;

/// What placement reads of one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellFacts {
    /// Land, not sea or lake.
    pub land: bool,
    /// `Cell.slope_milli_deg`.
    pub slope_milli_deg: u16,
    /// Passes logic/18 §site-place "entrance cell": land with no
    /// watercourse, road, land use, owner or building.
    pub free: bool,
}

/// A settlement that may anchor a dungeon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seat {
    /// Settlement id.
    pub id: SettlementId,
    /// Its cell.
    pub cell: (u32, u32),
    /// Its tier.
    pub rank: SeatRank,
}

/// Settlement tiers that can anchor a dungeon (hamlets never do).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatRank {
    /// One in ten gets a dungeon.
    Village,
    /// Always gets one.
    Town,
    /// Always gets one, at least two levels deep.
    City,
}

/// A site before its levels are fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chosen {
    /// Dungeon or cave.
    pub kind: Kind,
    /// Entrance cell.
    pub cell: (u32, u32),
    /// The settlement a dungeon hangs on.
    pub settlement: Option<SettlementId>,
    /// Fewest levels it may have.
    pub min_levels: u32,
    /// Danger tier of level 1.
    pub base_tier: u8,
}

/// The site id: stable per kind and entrance cell.
#[must_use]
pub fn site_id(kind: Kind, gx: u32, gy: u32) -> SiteId {
    let k = u64::from(kind == Kind::Cave);
    SiteId(subseed(k, "arda-site id", &[u64::from(gx), u64::from(gy)]))
}

/// Every site of a `size` (cells wide, high) world, dungeons first, each
/// group in a fixed order.
#[must_use]
pub fn choose(
    seed: u64,
    size: (u32, u32),
    cell: &dyn Fn(u32, u32) -> CellFacts,
    seats: &[Seat],
) -> Vec<Chosen> {
    let inside = |x: u32, y: u32| x >= 1 && y >= 1 && x + 1 < size.0 && y + 1 < size.1;
    let base_tier = |(x, y): (u32, u32)| -> u8 {
        let near = seats.iter().any(|s| {
            let (dx, dy) = (
                i64::from(s.cell.0) - i64::from(x),
                i64::from(s.cell.1) - i64::from(y),
            );
            s.rank != SeatRank::Village && dx * dx + dy * dy <= NEAR_CELLS * NEAR_CELLS
        });
        if near {
            1
        } else {
            2
        }
    };
    let mut taken: BTreeSet<(u32, u32)> = BTreeSet::new();
    let mut out = Vec::new();
    for s in seats {
        let picked = s.rank != SeatRank::Village
            || subseed(seed, "arda-site village", &[s.id.get()]) % 1000 < VILLAGE_ODDS;
        if !picked {
            continue;
        }
        let reach = *RING.end();
        let (x0, y0) = (
            s.cell.0.saturating_sub(reach),
            s.cell.1.saturating_sub(reach),
        );
        let best = (y0..=s.cell.1 + reach)
            .flat_map(|y| (x0..=s.cell.0 + reach).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                RING.contains(&x.abs_diff(s.cell.0).max(y.abs_diff(s.cell.1)))
                    && inside(x, y)
                    && !taken.contains(&(x, y))
                    && cell(x, y).free
            })
            .min_by_key(|&(x, y)| {
                subseed(
                    seed,
                    "arda-site ring",
                    &[s.id.get(), u64::from(x), u64::from(y)],
                )
            });
        if let Some(at) = best {
            taken.insert(at);
            out.push(Chosen {
                kind: Kind::Dungeon,
                cell: at,
                settlement: Some(s.id),
                min_levels: if s.rank == SeatRank::City { 2 } else { 1 },
                base_tier: base_tier(at),
            });
        }
    }
    for ty in 0..size.1.div_ceil(TILE) {
        for tx in 0..size.0.div_ceil(TILE) {
            let steepest = (ty * TILE..((ty + 1) * TILE).min(size.1))
                .flat_map(|y| (tx * TILE..((tx + 1) * TILE).min(size.0)).map(move |x| (x, y)))
                .filter_map(|(x, y)| {
                    let c = cell(x, y);
                    c.land.then_some((c.slope_milli_deg, x, y))
                })
                // Steepest first; ties to the lowest (gy, gx).
                .min_by_key(|&(s, x, y)| (std::cmp::Reverse(s), y, x));
            let Some((slope, x, y)) = steepest else {
                continue;
            };
            if slope < STEEP_MILLI_DEG {
                continue;
            }
            let foot = [
                (x, y.wrapping_sub(1)),
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y + 1),
            ]
            .into_iter()
            .filter(|&(nx, ny)| nx < size.0 && ny < size.1 && inside(nx, ny))
            .filter(|at| !taken.contains(at))
            .filter_map(|(nx, ny)| {
                let c = cell(nx, ny);
                c.free.then_some((c.slope_milli_deg, ny, nx))
            })
            .min();
            let Some((foot_slope, fy, fx)) = foot else {
                continue;
            };
            let kept =
                subseed(seed, "arda-site cave", &[u64::from(tx), u64::from(ty)]) % 1000 < CAVE_ODDS;
            if foot_slope >= WALKABLE_MILLI_DEG || !kept {
                continue;
            }
            taken.insert((fx, fy));
            out.push(Chosen {
                kind: Kind::Cave,
                cell: (fx, fy),
                settlement: None,
                min_levels: 1,
                base_tier: base_tier((fx, fy)),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn flat(_: u32, _: u32) -> CellFacts {
        CellFacts {
            land: true,
            slope_milli_deg: 2_000,
            free: true,
        }
    }

    #[test]
    fn towns_and_cities_always_get_a_dungeon_and_hamlets_have_no_seat() {
        let seats = [
            Seat {
                id: SettlementId(1),
                cell: (100, 100),
                rank: SeatRank::Town,
            },
            Seat {
                id: SettlementId(2),
                cell: (300, 300),
                rank: SeatRank::City,
            },
        ];
        let got = choose(42, (512, 512), &flat, &seats);
        assert_eq!(got.len(), 2);
        for c in &got {
            assert_eq!(c.kind, Kind::Dungeon);
            assert_eq!(c.base_tier, 1);
            let seat = seats.iter().find(|s| Some(s.id) == c.settlement).unwrap();
            let far = c
                .cell
                .0
                .abs_diff(seat.cell.0)
                .max(c.cell.1.abs_diff(seat.cell.1));
            assert!((2..=16).contains(&far), "{far}");
            assert_eq!(
                c.min_levels,
                if seat.rank == SeatRank::City { 2 } else { 1 }
            );
        }
    }

    #[test]
    fn about_one_village_in_ten_gets_a_dungeon() {
        let seats: Vec<Seat> = (0..2000_u32)
            .map(|i| Seat {
                id: SettlementId(u64::from(i) + 1),
                cell: (20 + (i % 40) * 40, 20 + (i / 40) * 40),
                rank: SeatRank::Village,
            })
            .collect();
        let n = choose(42, (2048, 2048), &flat, &seats).len();
        assert!((150..=250).contains(&n), "{n}");
    }

    #[test]
    fn a_cave_sits_at_the_foot_of_a_steep_cell() {
        let steep = |x: u32, y: u32| CellFacts {
            land: true,
            slope_milli_deg: if (x % 32, y % 32) == (10, 10) {
                30_000
            } else {
                3_000
            },
            free: true,
        };
        let got = choose(42, (640, 640), &steep, &[]);
        // 400 tiles qualify; the hash keeps about 30 %.
        assert!((90..=150).contains(&got.len()), "{}", got.len());
        for c in &got {
            assert_eq!(c.kind, Kind::Cave);
            assert_eq!((c.settlement, c.base_tier, c.min_levels), (None, 2, 1));
            let (tx, ty) = (c.cell.0 / 32 * 32 + 10, c.cell.1 / 32 * 32 + 10);
            assert_eq!(c.cell.0.abs_diff(tx) + c.cell.1.abs_diff(ty), 1);
        }
    }

    #[test]
    fn nothing_is_placed_on_gentle_or_unfree_ground() {
        assert!(choose(42, (640, 640), &flat, &[]).is_empty());
        let taken = |_: u32, _: u32| CellFacts {
            land: true,
            slope_milli_deg: 30_000,
            free: false,
        };
        assert!(choose(42, (640, 640), &taken, &[]).is_empty());
        let seat = [Seat {
            id: SettlementId(1),
            cell: (100, 100),
            rank: SeatRank::Town,
        }];
        assert!(choose(42, (640, 640), &taken, &seat).is_empty());
    }

    #[test]
    fn a_town_nearby_lowers_a_caves_base_tier() {
        let steep = |x: u32, y: u32| CellFacts {
            land: true,
            slope_milli_deg: if (x % 32, y % 32) == (10, 10) {
                30_000
            } else {
                3_000
            },
            free: true,
        };
        let seat = [Seat {
            id: SettlementId(1),
            cell: (320, 320),
            rank: SeatRank::Town,
        }];
        let got = choose(42, (640, 640), &steep, &seat);
        for c in got.iter().filter(|c| c.kind == Kind::Cave) {
            let (dx, dy) = (i64::from(c.cell.0) - 320, i64::from(c.cell.1) - 320);
            assert_eq!(
                c.base_tier,
                if dx * dx + dy * dy <= 150 * 150 { 1 } else { 2 }
            );
        }
    }

    #[test]
    fn placement_is_deterministic_and_ids_are_per_cell_and_kind() {
        let seats = [Seat {
            id: SettlementId(1),
            cell: (100, 100),
            rank: SeatRank::Town,
        }];
        assert_eq!(
            choose(7, (512, 512), &flat, &seats),
            choose(7, (512, 512), &flat, &seats)
        );
        assert_ne!(site_id(Kind::Cave, 3, 4), site_id(Kind::Dungeon, 3, 4));
        assert_ne!(site_id(Kind::Cave, 3, 4), site_id(Kind::Cave, 4, 3));
    }
}
