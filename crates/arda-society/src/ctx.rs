//! Validated input plus derived lookups shared by every stage.

use crate::buildings::{self, BuildingRef};
use crate::error::SocietyError;
use crate::graph::Graph;
use crate::input::{Settlement, WorldSettlements};
use crate::tables::Tables;
use std::collections::{BTreeMap, BTreeSet};

/// One realm with node indices resolved.
#[derive(Debug, Clone)]
pub struct RealmInfo {
    /// Realm id.
    pub id: u64,
    /// Realm name.
    pub name: String,
    /// Seat node index.
    pub seat: usize,
    /// Member node indices, sorted.
    pub members: Vec<usize>,
}

/// Shared, read-only simulation context.
#[derive(Debug)]
pub struct Ctx<'a> {
    /// World seed.
    pub seed: u64,
    /// Input world.
    pub world: &'a WorldSettlements,
    /// Data tables.
    pub t: &'a Tables,
    /// Road graph (node index = settlement input order).
    pub graph: Graph,
    /// Buildings per settlement id.
    pub buildings: BTreeMap<u64, Vec<BuildingRef>>,
    /// Realms sorted by id.
    pub realms: Vec<RealmInfo>,
    /// Present-day history span in years (year 1 to `present`).
    pub present: i32,
    /// Realm index of every node (review round 2 #29: a scan of the
    /// realms per lookup made every per-edge realm test O(R)).
    node_realm: Vec<usize>,
    /// Whether each node is a realm seat.
    seat: Vec<bool>,
}

/// Buildings a settlement may list beyond one per inhabitant.
pub const MAX_BUILDING_SLACK: u64 = 256;

impl<'a> Ctx<'a> {
    /// Validates `world` and builds the lookups.
    ///
    /// # Errors
    /// [`SocietyError::Input`] for duplicate ids, unknown realms, seats
    /// outside their realm, roads to unknown settlements, unknown biomes,
    /// building counts above population + [`MAX_BUILDING_SLACK`], or
    /// explicit buildings that are duplicated or belong to no settlement.
    pub fn new(
        seed: u64,
        world: &'a WorldSettlements,
        t: &'a Tables,
    ) -> Result<Self, SocietyError> {
        let mut ids = BTreeSet::new();
        for s in &world.settlements {
            if !ids.insert(s.id) {
                return Err(SocietyError::Input(format!(
                    "duplicate settlement id {}",
                    s.id
                )));
            }
            // An unknown biome would silently lose its yield modifiers.
            if !t.goods.biomes.contains_key(&s.biome) {
                return Err(SocietyError::Input(format!(
                    "settlement {} has unknown biome `{}`",
                    s.id, s.biome
                )));
            }
            // Derived buildings are materialised one by one; refuse counts
            // no settlement of this population could hold (§8: refuse,
            // never trim or exhaust memory).
            let total = s
                .buildings
                .values()
                .fold(0_u64, |a, &c| a.saturating_add(u64::from(c)));
            let cap = u64::from(s.population) + MAX_BUILDING_SLACK;
            if total > cap {
                return Err(SocietyError::Input(format!(
                    "settlement {} lists {total} buildings for {} people (limit {cap})",
                    s.id, s.population
                )));
            }
        }
        let mut building_ids = BTreeSet::new();
        for b in &world.buildings {
            if !ids.contains(&b.settlement_id) {
                return Err(SocietyError::Input(format!(
                    "building {} belongs to unknown settlement {}",
                    b.id, b.settlement_id
                )));
            }
            if !building_ids.insert((b.settlement_id, b.id)) {
                return Err(SocietyError::Input(format!(
                    "duplicate building id {} in settlement {}",
                    b.id, b.settlement_id
                )));
            }
        }
        let mut road_ids = BTreeSet::new();
        for r in &world.roads {
            if !road_ids.insert(r.id) {
                return Err(SocietyError::Input(format!("duplicate road id {}", r.id)));
            }
            let to_ok = r.to.is_none_or(|t| ids.contains(&t));
            if !ids.contains(&r.from) || !to_ok {
                return Err(SocietyError::Input(format!(
                    "road {} joins an unknown settlement",
                    r.id
                )));
            }
        }
        let graph = Graph::build(world);
        let realms = realms(world, &graph)?;
        let buildings = buildings::resolve(world, &t.goods);
        let span = 200 + crate::rng::hash(seed, "span", 0, 0) % 301;
        let present = crate::num::i32_of(crate::num::i64_of(span));
        let node_realm = world
            .settlements
            .iter()
            .map(|s| realm_ix_in(&realms, s.realm_id).unwrap_or(0))
            .collect();
        let mut seat = vec![false; world.settlements.len()];
        for r in &realms {
            if let Some(slot) = seat.get_mut(r.seat) {
                *slot = true;
            }
        }
        Ok(Self {
            seed,
            world,
            t,
            graph,
            buildings,
            realms,
            present,
            node_realm,
            seat,
        })
    }

    /// Settlement at node `i`.
    #[must_use]
    pub fn s(&self, i: usize) -> &Settlement {
        &self.world.settlements[i]
    }

    /// Number of settlements.
    #[must_use]
    pub fn n(&self) -> usize {
        self.world.settlements.len()
    }

    /// Node index of settlement `id`.
    #[must_use]
    pub fn node(&self, id: u64) -> Option<usize> {
        self.graph.index.get(&id).copied()
    }

    /// Index into [`Ctx::realms`] of realm `id`.
    #[must_use]
    pub fn realm_ix(&self, id: u64) -> Option<usize> {
        realm_ix_in(&self.realms, id)
    }

    /// Realm index of node `i`.
    #[must_use]
    pub fn realm_of(&self, i: usize) -> usize {
        self.node_realm.get(i).copied().unwrap_or(0)
    }

    /// Whether node `i` is a realm seat.
    #[must_use]
    pub fn is_seat(&self, i: usize) -> bool {
        self.seat.get(i).copied().unwrap_or(false)
    }

    /// Buildings of node `i`.
    #[must_use]
    pub fn buildings_of(&self, i: usize) -> &[BuildingRef] {
        self.buildings.get(&self.s(i).id).map_or(&[], Vec::as_slice)
    }

    /// Roads joining two different realms, keyed by the realm-index pair
    /// `(low, high)`, each list sorted by road id.
    #[must_use]
    pub fn border_roads(&self) -> BTreeMap<(usize, usize), Vec<u64>> {
        let mut out: BTreeMap<(usize, usize), Vec<u64>> = BTreeMap::new();
        for (u, edges) in self.graph.adj.iter().enumerate() {
            for e in edges {
                let (ra, rb) = (self.realm_of(u), self.realm_of(e.to));
                if ra < rb {
                    out.entry((ra, rb)).or_default().push(e.road);
                }
            }
        }
        for v in out.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        out
    }

    /// Whether node `i` has a road neighbour in another realm.
    #[must_use]
    pub fn is_border(&self, i: usize) -> bool {
        let r = self.realm_of(i);
        self.graph.adj[i].iter().any(|e| self.realm_of(e.to) != r)
    }

    /// Straight-line distance between two nodes, metres.
    #[must_use]
    pub fn dist(&self, a: usize, b: usize) -> u64 {
        let (sa, sb) = (self.s(a), self.s(b));
        crate::num::dist_m(sa.x_m, sa.y_m, sb.x_m, sb.y_m)
    }
}

/// Index of realm `id` in `realms`, which is sorted by id with unique ids.
fn realm_ix_in(realms: &[RealmInfo], id: u64) -> Option<usize> {
    realms.binary_search_by_key(&id, |r| r.id).ok()
}

fn realms(world: &WorldSettlements, graph: &Graph) -> Result<Vec<RealmInfo>, SocietyError> {
    let mut members: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (i, s) in world.settlements.iter().enumerate() {
        members.entry(s.realm_id).or_default().push(i);
    }
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for r in &world.realms {
        if !seen.insert(r.id) {
            return Err(SocietyError::Input(format!("duplicate realm id {}", r.id)));
        }
        let Some(&seat) = graph.index.get(&r.seat) else {
            return Err(SocietyError::Input(format!(
                "realm {} seat {} unknown",
                r.id, r.seat
            )));
        };
        if world.settlements[seat].realm_id != r.id {
            return Err(SocietyError::Input(format!(
                "realm {} seat lies outside it",
                r.id
            )));
        }
        let derived = members.remove(&r.id).unwrap_or_default();
        if !r.members.is_empty() {
            // The listed members must be exactly the settlements whose
            // `realm_id` names this realm: the partition is settle's.
            let mut listed: Vec<u64> = r.members.clone();
            listed.sort_unstable();
            let mut have: Vec<u64> = derived.iter().map(|&i| world.settlements[i].id).collect();
            have.sort_unstable();
            if listed != have {
                return Err(SocietyError::Input(format!(
                    "realm {} lists {} members but {} settlements name it",
                    r.id,
                    listed.len(),
                    have.len()
                )));
            }
        }
        out.push(RealmInfo {
            id: r.id,
            name: r.name.clone(),
            seat,
            members: derived,
        });
    }
    // `logic/06` branch: realms missing from the file get the largest member
    // as seat, so every settlement still belongs to a realm.
    for (id, list) in members {
        let seat = list
            .iter()
            .copied()
            .max_by_key(|&i| (world.settlements[i].population, std::cmp::Reverse(i)))
            .ok_or_else(|| SocietyError::Input(format!("realm {id} is empty")))?;
        out.push(RealmInfo {
            id,
            name: format!("{} March", world.settlements[seat].name),
            seat,
            members: list,
        });
    }
    out.sort_by_key(|r| r.id);
    Ok(out)
}
