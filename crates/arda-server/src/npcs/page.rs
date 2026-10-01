//! The world-people query engine (goal 57; logic/13 §npc-queries,
//! logic/16 §api-routes): filters over settlements' skeletons, expanding
//! only the people a page returns.
//!
//! Scale (goal 56): stored data stays O(settlements). A page builds at most
//! [`MAX_SCANNED`] settlement skeletons (a few integers per inhabitant,
//! dropped after the page) and expands at most `limit` people; no query
//! materialises commoners world-wide. Order is settlement id, then skeleton
//! order (home building id, resident index), so pages are deterministic and
//! a cursor resumes exactly where the previous page stopped.

use super::dto::{EntryOut, PageOut, NPC_PAGE_FORMAT};
use super::refs::{Cursor, NpcRef};
use crate::error::{ServerError, ServerResult};
use crate::people::people_error;
use arda_npc::{BuildingId, BuildingSpec, Generator, NotableSlot, Resident, SettlementProfile};
use arda_people::World;

/// Default page size.
pub const DEFAULT_LIMIT: usize = 50;
/// Largest page size (full SRD sheets are a few kB each).
pub const MAX_LIMIT: usize = 200;
/// Most settlement skeletons one page builds.
pub const MAX_SCANNED: u32 = 16;

/// Which link to a building a query follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    /// Lives there.
    Residents,
    /// Works there.
    Workers,
    /// Lives or works there.
    Either,
}

/// A parsed query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    /// Only this settlement.
    pub settlement: Option<u64>,
    /// Only people linked to this building (needs `settlement`).
    pub building: Option<(u64, Link)>,
    /// Only this occupation key (`smith`, `farmer`, …).
    pub job: Option<String>,
    /// Only settlements of this realm.
    pub realm: Option<u64>,
    /// Only notables (`true`) or only commoners (`false`).
    pub notable: Option<bool>,
    /// Page size, `1..=MAX_LIMIT`.
    pub limit: usize,
    /// Where to resume.
    pub cursor: Option<Cursor>,
}

impl Filter {
    fn keeps(&self, r: &Resident) -> bool {
        let building = self.building.is_none_or(|(b, link)| {
            let b = BuildingId(b);
            match link {
                Link::Residents => r.home == b,
                Link::Workers => r.workplace == Some(b),
                Link::Either => r.home == b || r.workplace == Some(b),
            }
        });
        building
            && self.job.as_deref().is_none_or(|j| r.job == j)
            && self.notable.is_none_or(|n| r.notable == n)
    }
}

/// A settlement's generator inputs, owned.
pub struct Inputs {
    /// Its profile.
    pub profile: SettlementProfile,
    /// Its buildings (plan or mix).
    pub buildings: Vec<BuildingSpec>,
    /// Society's notable slots.
    pub slots: Vec<NotableSlot>,
}

impl Inputs {
    /// Reads the inputs of settlement `id`.
    ///
    /// # Errors
    /// Unknown settlement (404) or world failures.
    pub fn of(w: &World, id: u64) -> ServerResult<Self> {
        Ok(Self {
            profile: w.profile(id).map_err(people_error)?,
            buildings: w.buildings(id).map_err(people_error)?.0,
            slots: w.slots(id).map_err(people_error)?,
        })
    }

    /// The skeleton, as `arda society build` built it.
    ///
    /// # Errors
    /// The generator's refusals.
    pub fn generator(&self, seed: u64) -> ServerResult<Generator<'_>> {
        Ok(Generator::with_notables(
            seed,
            &self.profile,
            &self.buildings,
            &self.slots,
        )?)
    }
}

/// The settlements a filter visits, in id order.
fn candidates(w: &World, f: &Filter) -> ServerResult<Vec<u64>> {
    let realm_ok = |id: u64| -> ServerResult<bool> {
        let (s, _) = w.files.settlement(id).map_err(people_error)?;
        Ok(f.realm.is_none_or(|r| s.realm_id.get() == r))
    };
    match f.settlement {
        Some(id) => Ok(if realm_ok(id)? { vec![id] } else { Vec::new() }),
        None => {
            let mut out = Vec::new();
            for &id in w.files.index.keys() {
                if realm_ok(id)? {
                    out.push(id);
                }
            }
            Ok(out)
        }
    }
}

/// Runs one page of `f` over world `w`.
///
/// # Errors
/// 404 for an unknown settlement or building; 400 for a cursor that does
/// not belong to this query; generator failures.
pub fn page(w: &World, f: &Filter) -> ServerResult<PageOut> {
    let list = candidates(w, f)?;
    let (first, mut position) = match f.cursor {
        None => (0, 0),
        Some(c) => (
            list.binary_search(&c.settlement).map_err(|_| {
                ServerError::BadRequest(format!("cursor {c} does not belong to this query"))
            })?,
            c.position,
        ),
    };
    let mut out = PageOut {
        format_version: NPC_PAGE_FORMAT,
        npcs: Vec::new(),
        next_cursor: None,
        scanned_settlements: 0,
    };
    for &id in list.iter().skip(first) {
        if out.scanned_settlements == MAX_SCANNED {
            out.next_cursor = Some(
                Cursor {
                    settlement: id,
                    position: 0,
                }
                .to_string(),
            );
            break;
        }
        out.scanned_settlements += 1;
        let inputs = Inputs::of(w, id)?;
        let g = inputs.generator(w.seed())?;
        let range = match f.building {
            Some((b, link)) => {
                let r = g.residents_of(BuildingId(b)).ok_or_else(|| {
                    ServerError::NotFound(format!("settlement {id} has no building {b}"))
                })?;
                if link == Link::Residents {
                    r
                } else {
                    0..g.len()
                }
            }
            None => 0..g.len(),
        };
        for p in range.start.max(position)..range.end {
            let Some(r) = g.resident(p) else { break };
            if !f.keeps(&r) {
                continue;
            }
            if out.npcs.len() == f.limit {
                out.next_cursor = Some(
                    Cursor {
                        settlement: id,
                        position: p,
                    }
                    .to_string(),
                );
                return Ok(out);
            }
            out.npcs.push(EntryOut {
                npc_ref: NpcRef {
                    settlement: id,
                    building: r.home.get(),
                    index: r.index,
                }
                .to_string(),
                settlement_id: id.to_string(),
                notable: r.notable,
                npc: g.npc_at(p)?,
            });
        }
        position = 0;
    }
    Ok(out)
}
