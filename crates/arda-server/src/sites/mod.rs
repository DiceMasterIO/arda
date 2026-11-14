//! Dungeon and cave sites of the served world (logic/18 §site-place,
//! §site-levels): placed once when the server opens, held in memory.

pub mod dto;
pub mod place;
pub mod routes;

use crate::error::ServerResult;
use crate::query::WorldQuery;
use arda_dungeon::site::{danger_tier, level_seed, level_sizes, MAX_ATTEMPTS};
use arda_dungeon::{DungeonError, Kind, Params};
use arda_ids::SettlementId;
use arda_settle::model::{Settlement, Tier};
use arda_tactical::Library;
use place::{CellFacts, Chosen, Seat, SeatRank};
use rayon::prelude::*;

/// A site's id: stable as long as the site keeps its kind and entrance
/// cell. A decimal string on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SiteId(pub u64);

impl std::fmt::Display for SiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for SiteId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

/// One level of a site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelRef {
    /// 1-based depth.
    pub level: u32,
    /// The seed that generates it.
    pub seed: u64,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// SRD level band, 1 to 4.
    pub danger_tier: u8,
}

/// A placed site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// Stable id.
    pub id: SiteId,
    /// Dungeon or cave.
    pub kind: Kind,
    /// Entrance cell.
    pub cell: (u32, u32),
    /// The settlement a dungeon hangs on.
    pub settlement: Option<SettlementId>,
    /// 1 to 3 levels, in depth order.
    pub levels: Vec<LevelRef>,
}

impl Site {
    /// The entrance square in world squares: the cell's centre square.
    #[must_use]
    pub fn entrance_gs(&self) -> [i64; 2] {
        let n = arda_blocks::SQUARES_PER_CELL;
        [
            i64::from(self.cell.0) * n + n / 2,
            i64::from(self.cell.1) * n + n / 2,
        ]
    }
}

/// The world's sites, ordered by id.
#[derive(Debug, Default)]
pub struct Sites(Vec<Site>);

impl Sites {
    /// No sites.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Every site, by id.
    #[must_use]
    pub fn all(&self) -> &[Site] {
        &self.0
    }

    /// One site.
    #[must_use]
    pub fn get(&self, id: SiteId) -> Option<&Site> {
        self.0
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .map(|i| &self.0[i])
    }

    /// Places the world's sites and fixes every level's seed by generating
    /// the level and its scene (logic/18 §site-levels): a seed that fails
    /// is re-derived up to [`MAX_ATTEMPTS`] times, and a site with a level
    /// that never generates is not placed.
    ///
    /// # Errors
    /// The world's areas cannot be read.
    pub fn place(
        query: &WorldQuery,
        settlements: Option<&[Settlement]>,
        library: &Library,
    ) -> ServerResult<Self> {
        let seed = query.world().seed();
        let size = query.cells();
        let n = u32::from(arda_core::AREA_CELLS);
        // ceiling: one sequential pass over every area through the query's
        // bounded area cache, and one `CellFacts` per cell held until the
        // sites are chosen (about 200 MB for a 5,000 × 10,000 cell world).
        // Place per area, in parallel, if a default world's start grows
        // past a few seconds (OI-63).
        let mut facts: Vec<CellFacts> = Vec::with_capacity(size.0 as usize * size.1 as usize);
        facts.resize(
            size.0 as usize * size.1 as usize,
            CellFacts {
                land: false,
                slope_milli_deg: 0,
                free: false,
            },
        );
        for ay in 0..size.1.div_ceil(n) {
            for ax in 0..size.0.div_ceil(n) {
                let area = query.area(
                    i32::try_from(ax).unwrap_or(i32::MAX),
                    i32::try_from(ay).unwrap_or(i32::MAX),
                )?;
                for y in 0..n.min(size.1 - ay * n) {
                    for x in 0..n.min(size.0 - ax * n) {
                        let c = area.cell(
                            u16::try_from(x).unwrap_or(u16::MAX),
                            u16::try_from(y).unwrap_or(u16::MAX),
                        )?;
                        let (gx, gy) = (ax * n + x, ay * n + y);
                        let land = c.terrain == arda_core::TerrainKind::Land;
                        let soc = query.society().map(|s| s.at(gx, gy));
                        let free = land
                            && c.watercourse_order == 0
                            && c.road == arda_core::RoadClass::None
                            && c.built_by == 0
                            && soc.is_none_or(|s| {
                                s.land_use == 0 && s.owner == 0 && s.road.unwrap_or(0) == 0
                            });
                        facts[(gy * size.0 + gx) as usize] = CellFacts {
                            land,
                            slope_milli_deg: c.slope_milli_deg,
                            free,
                        };
                    }
                }
            }
        }
        let seats: Vec<Seat> = settlements
            .unwrap_or_default()
            .iter()
            .filter_map(|s| {
                let rank = match s.tier {
                    Tier::Hamlet => return None,
                    Tier::Village => SeatRank::Village,
                    Tier::Town => SeatRank::Town,
                    Tier::City => SeatRank::City,
                };
                Some(Seat {
                    id: s.id,
                    cell: (s.cell_x, s.cell_y),
                    rank,
                })
            })
            .collect();
        let cell = |x: u32, y: u32| facts[(y * size.0 + x) as usize];
        // Each site's levels are generated and their scenes built
        // independently, so the sites are settled in parallel; the sort
        // below fixes the order.
        let mut sites: Vec<Site> = place::choose(seed, size, &cell, &seats)
            .into_par_iter()
            .filter_map(|c| build(seed, &c, library))
            .collect();
        sites.sort_by_key(|s| s.id);
        Ok(Self(sites))
    }
}

/// Whether `params` generates and builds the scene `/scene` serves: the
/// scene is built with the render seed of the level's name, as the level
/// and scene handlers build it (logic/18 §site-levels).
fn usable(params: &Params, world_seed: u64, library: &Library) -> bool {
    match arda_dungeon::generate(params) {
        Ok(d) => {
            let seed = crate::tactical::key::render_seed(
                world_seed,
                &crate::tactical::Anchor::Name(d.layout.name.clone()),
            );
            arda_scene::build_scene(&d.layout, library, seed, Some(&d.rules)).is_ok()
        }
        Err(DungeonError::Generation(_) | DungeonError::Params(_)) => false,
    }
}

/// The seed of the first attempt, in order, that `works`; `None` after
/// [`MAX_ATTEMPTS`] failures.
fn settle(seed: u64, id: u64, level: u32, works: &dyn Fn(u64) -> bool) -> Option<u64> {
    (0..MAX_ATTEMPTS)
        .map(|attempt| level_seed(seed, id, level, attempt))
        .find(|s| works(*s))
}

fn build(seed: u64, c: &Chosen, library: &Library) -> Option<Site> {
    let id = place::site_id(c.kind, c.cell.0, c.cell.1);
    let levels = level_sizes(id.0, c.min_levels)
        .into_iter()
        .zip(1_u32..)
        .map(|(size, level)| {
            let works = |s: u64| {
                let params = Params {
                    seed: s,
                    kind: c.kind,
                    width: size.0,
                    height: size.1,
                };
                usable(&params, seed, library)
            };
            settle(seed, id.0, level, &works).map(|s| LevelRef {
                level,
                seed: s,
                width: size.0,
                height: size.1,
                danger_tier: danger_tier(c.base_tier, level),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Site {
        id,
        kind: c.kind,
        cell: c.cell,
        settlement: c.settlement,
        levels,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn attempts_run_in_order_and_the_first_success_wins() {
        let tried = RefCell::new(Vec::new());
        let third = level_seed(42, 7, 1, 2);
        let got = settle(42, 7, 1, &|s| {
            tried.borrow_mut().push(s);
            s == third
        });
        assert_eq!(got, Some(third));
        let want: Vec<u64> = (0..3).map(|a| level_seed(42, 7, 1, a)).collect();
        assert_eq!(*tried.borrow(), want);
    }

    #[test]
    fn eight_failures_give_the_level_up() {
        let tried = RefCell::new(0_u32);
        assert_eq!(
            settle(42, 7, 1, &|_| {
                *tried.borrow_mut() += 1;
                false
            }),
            None
        );
        assert_eq!(*tried.borrow(), MAX_ATTEMPTS);
    }
}
