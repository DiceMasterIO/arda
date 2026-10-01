//! One world's settlement products, derived on demand and cached: town
//! plans, building lists, notable slots and populations (logic/13
//! §npc-regeneration: commoners are regenerated, never stored).

use crate::files::{self, NotablesFile, SocietyFiles, NOTABLES_FILE, SOCIETY_FILE};
use crate::shared::SharedSource;
use crate::town;
use crate::PeopleError;
use arda_npc::{BuildingSpec, Generator, NotableSlot, Npc, NpcId, Population, SettlementProfile};
use arda_refine::Source;
use arda_society::Society;
use arda_town::TownPlan;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Town plans kept in memory (each a few hundred kB).
const PLAN_CACHE: usize = 64;

/// Where a settlement's buildings come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingSource {
    /// Its `arda-town` plan.
    Plan,
    /// Its estimated building mix (no plan could be drawn).
    Mix,
}

impl BuildingSource {
    /// `plan` or `mix`.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Mix => "mix",
        }
    }
}

type PlanCache = BTreeMap<u64, (u64, Arc<Option<TownPlan>>)>;

/// A world with its `society/` directory.
pub struct World {
    /// The world's cells, shared with the tactical pipeline.
    pub src: SharedSource,
    /// The settlement stage's files.
    pub files: SocietyFiles,
    /// `society.json`, when `arda society build` has run.
    pub society: Option<Society>,
    /// `notables.json`, when `arda society build` has run.
    pub notables: Option<NotablesFile>,
    notable_index: BTreeMap<NpcId, (usize, usize)>,
    society_index: BTreeMap<u64, usize>,
    plans: Mutex<(u64, PlanCache)>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("settlements", &self.files.settlements.settlements.len())
            .field("society", &self.society.is_some())
            .finish_non_exhaustive()
    }
}

impl World {
    /// Opens the world at `dir` and its `society/` files.
    ///
    /// # Errors
    /// World, I/O or format errors; a missing `society/settlements.json`.
    pub fn open(dir: &Path) -> Result<Self, PeopleError> {
        Self::with_source(SharedSource::open(dir)?, &dir.join("society"))
    }

    /// A world over an opened source and a `society/` directory.
    ///
    /// # Errors
    /// I/O or format errors.
    pub fn with_source(src: SharedSource, society_dir: &Path) -> Result<Self, PeopleError> {
        let files = SocietyFiles::read(society_dir)?;
        let read_opt = |name: &str| {
            let p = society_dir.join(name);
            p.exists().then_some(p)
        };
        let society: Option<Society> = read_opt(SOCIETY_FILE)
            .map(|p| files::read(&p))
            .transpose()?;
        let notables: Option<NotablesFile> = read_opt(NOTABLES_FILE)
            .map(|p| files::read(&p))
            .transpose()?;
        let mut notable_index = BTreeMap::new();
        for (i, s) in notables
            .iter()
            .flat_map(|n| n.settlements.iter())
            .enumerate()
        {
            for (j, npc) in s.npcs.iter().enumerate() {
                notable_index.insert(npc.id, (i, j));
            }
        }
        let society_index = society
            .iter()
            .flat_map(|s| s.settlements.iter())
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        Ok(Self {
            src,
            files,
            society,
            notables,
            notable_index,
            society_index,
            plans: Mutex::new((0, BTreeMap::new())),
        })
    }

    /// The world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.src.seed()
    }

    fn lock_plans(&self) -> Result<std::sync::MutexGuard<'_, (u64, PlanCache)>, PeopleError> {
        let mut g = self
            .plans
            .lock()
            .map_err(|_| PeopleError::World("plan cache poisoned".into()))?;
        g.0 += 1;
        Ok(g)
    }

    /// The settlement's town plan (cached), or `None` without one.
    ///
    /// # Errors
    /// Unknown settlement, record or world failures.
    pub fn plan(&self, id: u64) -> Result<Arc<Option<TownPlan>>, PeopleError> {
        {
            let mut g = self.lock_plans()?;
            let tick = g.0;
            if let Some(hit) = g.1.get_mut(&id) {
                hit.0 = tick;
                return Ok(Arc::clone(&hit.1));
            }
        }
        let (s, record) = self.files.settlement(id)?;
        let plan = Arc::new(town::plan(&self.src, s, record, &self.files.roads.roads)?);
        let mut g = self.lock_plans()?;
        let tick = g.0;
        g.1.insert(id, (tick, Arc::clone(&plan)));
        while g.1.len() > PLAN_CACHE {
            let oldest = g.1.iter().min_by_key(|(_, e)| e.0).map(|(k, _)| *k);
            if let Some(k) = oldest {
                g.1.remove(&k);
            }
        }
        Ok(plan)
    }

    /// Draws and caches the plans of the most populous settlements of the
    /// tiers `keep` selects (at most half the plan cache), with their
    /// interior salts, so the first tactical request near a town or city
    /// does not wait for its plan (goal 50). Failures are skipped: a plan
    /// that cannot be drawn fails again, visibly, on request.
    pub fn warm_plans(&self, keep: impl Fn(arda_settle::model::Tier) -> bool + Sync) {
        use rayon::prelude::*;
        let mut ids: Vec<(u32, u64)> = self
            .files
            .settlements
            .settlements
            .iter()
            .filter(|s| keep(s.tier))
            .map(|s| (s.population, s.id.get()))
            .collect();
        ids.sort_unstable_by_key(|&(p, id)| (std::cmp::Reverse(p), id));
        ids.truncate(PLAN_CACHE / 2);
        ids.par_iter().for_each(|&(_, id)| {
            if let Ok(plan) = self.plan(id) {
                if let Some(b) = plan.as_ref().as_ref().and_then(|p| p.buildings.first()) {
                    let _ = plan.as_ref().as_ref().map(|p| p.interior_salt(b));
                }
            }
        });
    }

    /// [`Self::warm_plans`] for the towns and cities.
    pub fn warm_towns(&self) {
        use arda_settle::model::Tier;
        self.warm_plans(|t| matches!(t, Tier::Town | Tier::City));
    }

    /// The settlement's profile for the NPC generator (adapter A2: the
    /// record deserialises into it).
    ///
    /// # Errors
    /// Unknown settlement or a record that does not fit.
    pub fn profile(&self, id: u64) -> Result<SettlementProfile, PeopleError> {
        let (_, record) = self.files.settlement(id)?;
        serde_json::from_value(record.clone())
            .map_err(|e| PeopleError::format(&self.files.dir.join("settlements.json"), e))
    }

    fn society_of(&self, id: u64) -> Result<&arda_society::SettlementSociety, PeopleError> {
        let society = self.society.as_ref().ok_or_else(|| {
            PeopleError::World("society/society.json is missing: run `arda society build`".into())
        })?;
        self.society_index
            .get(&id)
            .and_then(|&i| society.settlements.get(i))
            .ok_or(PeopleError::UnknownSettlement(id))
    }

    /// The settlement's buildings: its plan's when it has one, else those
    /// derived from its mix (with the ids `arda-society` gave them).
    ///
    /// # Errors
    /// Unknown settlement, a missing `society.json` for a plan-less
    /// settlement, or world failures.
    pub fn buildings(&self, id: u64) -> Result<(Vec<BuildingSpec>, BuildingSource), PeopleError> {
        if let Some(plan) = self.plan(id)?.as_ref() {
            return Ok((
                town::plan_specs(plan, self.files.settlement(id)?.0.population),
                BuildingSource::Plan,
            ));
        }
        let (s, _) = self.files.settlement(id)?;
        let refs = &self.society_of(id)?.buildings;
        Ok((town::mix_specs(s, refs), BuildingSource::Mix))
    }

    /// Society's role slots of the settlement as notable slots (A14),
    /// empty before `arda society build`.
    ///
    /// # Errors
    /// Unknown settlement.
    pub fn slots(&self, id: u64) -> Result<Vec<NotableSlot>, PeopleError> {
        self.files.settlement(id)?;
        let Ok(s) = self.society_of(id) else {
            return Ok(Vec::new());
        };
        Ok(s.roles.iter().map(slot).collect())
    }

    /// The settlement's population: households, roster and notables, with
    /// commoners regenerated from the same inputs.
    ///
    /// # Errors
    /// As [`World::buildings`], and the NPC generator's refusals.
    pub fn population(&self, id: u64) -> Result<Population, PeopleError> {
        let profile = self.profile(id)?;
        let (buildings, _) = self.buildings(id)?;
        let slots = self.slots(id)?;
        let g = Generator::with_notables(self.seed(), &profile, &buildings, &slots)?;
        Ok(g.population()?)
    }

    /// A stored notable by id (`notables.json`).
    #[must_use]
    pub fn notable(&self, id: NpcId) -> Option<(&Npc, arda_ids::SettlementId)> {
        let (i, j) = *self.notable_index.get(&id)?;
        let s = self.notables.as_ref()?.settlements.get(i)?;
        Some((s.npcs.get(j)?, s.settlement_id))
    }

    /// Every stored notable of a settlement.
    #[must_use]
    pub fn notables_of(&self, id: u64) -> &[Npc] {
        self.notables
            .as_ref()
            .and_then(|n| n.settlements.iter().find(|s| s.settlement_id.get() == id))
            .map_or(&[], |s| s.npcs.as_slice())
    }
}

/// One society role as a notable slot.
#[must_use]
pub fn slot(r: &arda_society::roles::NpcRole) -> NotableSlot {
    NotableSlot {
        id: r.id.clone(),
        kind: r.kind.clone(),
        title: r.title.clone(),
        building: r.building.map(arda_npc::BuildingId),
        given_name: r.given_name.clone(),
        family_name: r.family_name.clone(),
        female: r.female,
    }
}
