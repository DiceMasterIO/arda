//! One world's settlement products, derived on demand and cached: town
//! plans, building lists, notable slots and populations (logic/13
//! §npc-regeneration: commoners are regenerated, never stored).

use crate::files::{self, SocietyFiles, NOTABLES_FILE, SOCIETY_FILE};
use crate::notables::Notables;
use crate::shared::SharedSource;
use crate::town;
use crate::PeopleError;
use arda_npc::{BuildingSpec, Generator, NotableSlot, Npc, NpcId, Population, SettlementProfile};
use arda_refine::Source;
use arda_society::Society;
use arda_town::TownPlan;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Town plans kept in memory, at most.
pub const PLAN_CACHE: usize = 64;
/// Bytes of town plans held in memory: cached ([`plan_bytes`] each) and
/// being drawn ([`plan_estimate`] each, reserved before drawing). A
/// village's plan is a few MB; a default-size world's largest city holds a
/// half-gigabyte square grid, so the count alone does not bound memory.
/// The most recently drawn plan stays even when it alone is larger, and a
/// plan whose estimate alone is larger is drawn when no other is.
pub const PLAN_CACHE_BYTES: u64 = 768 << 20;
/// [`plan_estimate`]'s bytes a person.
pub const PLAN_BYTES_PER_PERSON: u64 = 8 << 10;
/// [`plan_estimate`]'s bytes for any plan.
pub const PLAN_FLOOR_BYTES: u64 = 16 << 20;

/// The bytes reserved while `s`'s plan is drawn: [`PLAN_BYTES_PER_PERSON`]
/// a person plus [`PLAN_FLOOR_BYTES`]. Seed-42 plans stay under it: MICRO
/// hamlets to 5.6 MB, villages to 16.1 MB, towns and cities 4 to 7.3 KiB
/// a person; the default-size world's largest city, 91,535 people, is
/// 509 MB against 767 MB.
#[must_use]
pub fn plan_estimate(s: &arda_settle::model::Settlement) -> u64 {
    u64::from(s.population) * PLAN_BYTES_PER_PERSON + PLAN_FLOOR_BYTES
}
/// Population skeletons kept in memory (a city's is a few MB).
const GENERATOR_CACHE: usize = 16;

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

/// Settlement id to (last-use tick, plan, [`plan_bytes`]).
type PlanCache = BTreeMap<u64, (u64, Arc<Option<TownPlan>>, u64)>;

/// The plan cache and the bytes reserved by plans being drawn.
struct Plans {
    tick: u64,
    cache: PlanCache,
    drawing: u64,
    /// [`PLAN_CACHE_BYTES`], or a test's smaller budget.
    budget: u64,
}

impl Plans {
    /// Evicts least recently used plans (sparing the newest when `spare`)
    /// while the cache holds too many, or it and the draws, plus `extra`,
    /// reserve more than the budget.
    fn evict(&mut self, extra: u64, spare: bool) {
        let floor = usize::from(spare);
        while self.cache.len() > floor
            && (self.cache.len() > PLAN_CACHE
                || cache_bytes(&self.cache) + self.drawing + extra > self.budget)
        {
            let oldest = self.cache.iter().min_by_key(|(_, e)| e.0).map(|(k, _)| *k);
            if let Some(k) = oldest {
                self.cache.remove(&k);
            }
        }
    }
}

/// Runs `draw` (a plan draw, parallel inside) in a pool of its own, from a
/// thread outside every rayon pool. [`World::plan`] can wait for another
/// draw's reservation, so a draw must never run where that wait can be
/// stacked on top of it: a rayon worker drawing, or helping a draw, steals
/// other jobs while it joins, and one of them may be a `plan` call that
/// waits for the very draw below it on the same stack. The draw pool runs
/// only draw work, and a plain thread waits for it without stealing.
fn draw_apart<T: Send>(draw: impl FnOnce() -> T + Send) -> Result<T, PeopleError> {
    static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
    let pool = POOL
        .get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .thread_name(|i| format!("arda-plan-{i}"))
                .build()
                .ok()
        })
        .as_ref()
        .ok_or_else(|| PeopleError::World("the plan-drawing threads could not start".into()))?;
    Ok(std::thread::scope(|scope| {
        scope
            .spawn(|| pool.install(draw))
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    }))
}

/// A draw's reservation, released when dropped (also on failure).
struct Reserved<'a> {
    world: &'a World,
    bytes: u64,
}

impl Drop for Reserved<'_> {
    fn drop(&mut self) {
        let mut g = self
            .world
            .plans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        g.drawing = g.drawing.saturating_sub(self.bytes);
        drop(g);
        self.world.drawn.notify_all();
    }
}

/// Resident bytes charged for a town plan: its square grid exactly (it is
/// not serialised) plus twice its JSON for everything else.
#[must_use]
pub fn plan_bytes(plan: &TownPlan) -> u64 {
    /// Counts bytes written, keeping none.
    struct Count(u64);
    impl std::io::Write for Count {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 += buf.len() as u64;
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let g = &plan.grid;
    let grid = std::mem::size_of_val(g.kind.as_slice())
        + std::mem::size_of_val(g.street.as_slice())
        + std::mem::size_of_val(g.plot.as_slice())
        + std::mem::size_of_val(g.building.as_slice())
        + std::mem::size_of_val(g.height.as_slice())
        + std::mem::size_of_val(g.slope.as_slice())
        + std::mem::size_of_val(g.depth.as_slice());
    let mut json = Count(0);
    // Serialising into a counter cannot fail; a failure charges the grid.
    let _ = serde_json::to_writer(&mut json, plan);
    (grid as u64).saturating_add(json.0.saturating_mul(2))
}
type GeneratorCache = BTreeMap<u64, (u64, Arc<Generator<'static>>)>;

/// A world with its `society/` directory.
pub struct World {
    /// The world's cells, shared with the tactical pipeline.
    pub src: SharedSource,
    /// The settlement stage's files.
    pub files: SocietyFiles,
    /// `society.json`, when `arda society build` has run.
    pub society: Option<Society>,
    /// `notables.json`, when `arda society build` has run, read lazily
    /// (review 3.3-10).
    pub notables: Option<Notables>,
    society_index: BTreeMap<u64, usize>,
    plans: Mutex<Plans>,
    /// Signalled when a draw releases its reservation.
    drawn: Condvar,
    generators: Mutex<(u64, GeneratorCache)>,
    /// Population skeletons built so far (cache misses of [`World::generator`]).
    skeletons_built: AtomicU64,
    /// The largest [`plan_bytes`] drawn since opening.
    largest_plan: AtomicU64,
    /// Settlements whose plan, drawn again here, does not match the plan
    /// their stored notables were generated in (review 3.3-5).
    stale_plans: Mutex<BTreeSet<u64>>,
}

impl std::fmt::Debug for World {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World")
            .field("settlements", &self.files.settlements.settlements.len())
            .field("society", &self.society.is_some())
            .finish_non_exhaustive()
    }
}

fn cache_bytes(cache: &PlanCache) -> u64 {
    cache.values().map(|e| e.2).fold(0, u64::saturating_add)
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
        let notables: Option<Notables> = read_opt(NOTABLES_FILE)
            .map(|p| Notables::open(&p))
            .transpose()?;
        check_stamps(society_dir, &files, society.as_ref(), notables.as_ref())?;
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
            society_index,
            plans: Mutex::new(Plans {
                tick: 0,
                cache: BTreeMap::new(),
                drawing: 0,
                budget: PLAN_CACHE_BYTES,
            }),
            drawn: Condvar::new(),
            generators: Mutex::new((0, BTreeMap::new())),
            skeletons_built: AtomicU64::new(0),
            largest_plan: AtomicU64::new(0),
            stale_plans: Mutex::new(BTreeSet::new()),
        })
    }

    /// The world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.src.seed()
    }

    fn lock_plans(&self) -> Result<std::sync::MutexGuard<'_, Plans>, PeopleError> {
        self.plans
            .lock()
            .map_err(|_| PeopleError::World("plan cache poisoned".into()))
    }

    /// The settlement's town plan (cached), or `None` without one.
    ///
    /// A plan not cached is drawn once [`plan_estimate`] bytes are
    /// reserved within [`PLAN_CACHE_BYTES`] beside the cache and the other
    /// draws (evicting cached plans first); otherwise the call waits for a
    /// draw to finish. A plan larger than the whole budget is drawn alone.
    ///
    /// # Errors
    /// Unknown settlement, record or world failures.
    pub fn plan(&self, id: u64) -> Result<Arc<Option<TownPlan>>, PeopleError> {
        let (s, record) = self.files.settlement(id)?;
        let estimate = plan_estimate(s);
        let reserved = {
            let mut g = self.lock_plans()?;
            loop {
                g.tick += 1;
                let tick = g.tick;
                if let Some(hit) = g.cache.get_mut(&id) {
                    hit.0 = tick;
                    return Ok(Arc::clone(&hit.1));
                }
                g.evict(estimate, false);
                if g.drawing == 0 || cache_bytes(&g.cache) + g.drawing + estimate <= g.budget {
                    g.drawing += estimate;
                    break Reserved {
                        world: self,
                        bytes: estimate,
                    };
                }
                g = self
                    .drawn
                    .wait(g)
                    .map_err(|_| PeopleError::World("plan cache poisoned".into()))?;
            }
        };
        let mut plan = draw_apart(|| town::plan(&self.src, s, record, &self.files.roads.roads))??;
        if let (Some(p), Some(society)) = (plan.as_mut(), self.society.as_ref()) {
            town::read_history(p, society);
        }
        self.check_plan(id, plan.as_ref())?;
        let bytes = plan.as_ref().map_or(0, plan_bytes);
        self.largest_plan.fetch_max(bytes, Ordering::Relaxed);
        let plan = Arc::new(plan);
        {
            let mut g = self.lock_plans()?;
            g.tick += 1;
            let tick = g.tick;
            g.cache.insert(id, (tick, Arc::clone(&plan), bytes));
            // The plan now stands in for this draw's reservation, which
            // goes when `reserved` drops; the newest entry is spared.
            g.drawing = g.drawing.saturating_sub(estimate);
            g.evict(0, true);
            g.drawing += estimate;
        }
        drop(reserved);
        Ok(plan)
    }

    /// Sum of the cached plans' [`plan_bytes`].
    ///
    /// # Errors
    /// A poisoned cache.
    pub fn cached_plan_bytes(&self) -> Result<u64, PeopleError> {
        Ok(cache_bytes(&self.lock_plans()?.cache))
    }

    /// Replaces [`PLAN_CACHE_BYTES`] for this world (tests of the bound).
    #[doc(hidden)]
    pub fn set_plan_budget(&self, bytes: u64) {
        if let Ok(mut g) = self.lock_plans() {
            g.budget = bytes;
        }
    }

    /// Bytes reserved by plans being drawn now.
    ///
    /// # Errors
    /// A poisoned cache.
    pub fn drawing_plan_bytes(&self) -> Result<u64, PeopleError> {
        Ok(self.lock_plans()?.drawing)
    }

    /// The largest [`plan_bytes`] drawn since opening, cached or not.
    #[must_use]
    pub fn largest_plan_drawn(&self) -> u64 {
        self.largest_plan.load(Ordering::Relaxed)
    }

    /// Draws and caches the plans of the most populous settlements of the
    /// tiers `keep` selects, with their interior salts, so the first
    /// tactical request near a town or city does not wait for its plan
    /// (goal 50). Plans are drawn most populous first, in batches drawn
    /// together (each plan still parallel inside) until the cache holds
    /// half its count or half its bytes ([`PLAN_CACHE_BYTES`]). A batch is
    /// the longest run of the next plans (at least one) that reads at most
    /// `max_areas` distinct areas ([`World::plan_areas`]), has at most one
    /// plan per rayon thread, and whose [`plan_estimate`]s fit the budget
    /// beside the cache, so its draws need not wait for each other.
    /// Failures are skipped: a plan that cannot be drawn fails again,
    /// visibly, on request.
    pub fn warm_plans(
        &self,
        keep: impl Fn(arda_settle::model::Tier) -> bool + Sync,
        max_areas: usize,
    ) {
        use rayon::prelude::*;
        let mut todo: Vec<_> = self
            .files
            .settlements
            .settlements
            .iter()
            .filter(|s| keep(s.tier))
            .map(|s| {
                let reads = self.plan_areas(s);
                (s.population, s.id.get(), plan_estimate(s), reads)
            })
            .collect();
        todo.sort_unstable_by_key(|&(p, id, _, _)| (std::cmp::Reverse(p), id));
        todo.truncate(PLAN_CACHE / 2);
        let threads = rayon::current_num_threads().max(1);
        let mut rest = todo.as_slice();
        while !rest.is_empty() {
            let room = match self.cached_plan_bytes() {
                Ok(bytes) if bytes < PLAN_CACHE_BYTES / 2 => PLAN_CACHE_BYTES - bytes,
                _ => break,
            };
            let mut areas = BTreeSet::new();
            let (mut n, mut estimate) = (0, 0_u64);
            for (_, _, bytes, reads) in rest {
                let joined: BTreeSet<_> = areas.union(reads).copied().collect();
                let more = estimate + bytes;
                if n > 0 && (joined.len() > max_areas || n >= threads || more > room) {
                    break;
                }
                (areas, estimate) = (joined, more);
                n += 1;
            }
            let (batch, later) = rest.split_at(n);
            batch.par_iter().for_each(|&(_, id, _, _)| {
                if let Ok(plan) = self.plan(id) {
                    if let Some(b) = plan.as_ref().as_ref().and_then(|p| p.buildings.first()) {
                        let _ = plan.as_ref().as_ref().map(|p| p.interior_salt(b));
                    }
                }
            });
            rest = later;
        }
    }

    /// [`Self::warm_plans`] for the towns and cities.
    pub fn warm_towns(&self, max_areas: usize) {
        use arda_settle::model::Tier;
        self.warm_plans(|t| matches!(t, Tier::Town | Tier::City), max_areas);
    }

    /// The areas, as (column, row), whose cells drawing `s`'s plan reads:
    /// those within its plan radius plus the terrain margin
    /// ([`town::TERRAIN_MARGIN_M`]) and one cell.
    #[must_use]
    pub fn plan_areas(&self, s: &arda_settle::model::Settlement) -> BTreeSet<(i64, i64)> {
        use crate::terrain::CELL_M;
        let reach = town::plan_radius_for(s) + town::TERRAIN_MARGIN_M + CELL_M;
        let (w, h) = self.src.cells_wide_high();
        let per_area = i64::from(arda_core::AREA_CELLS);
        // World metres fit i64 and f64 exactly.
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let area = |m: i64, d: f64, cells: i64| {
            let cell = ((m as f64 + d) / CELL_M).floor() as i64;
            cell.clamp(0, cells - 1).div_euclid(per_area)
        };
        let (x0, x1) = (area(s.x_m, -reach, w), area(s.x_m, reach, w));
        let (y0, y1) = (area(s.y_m, -reach, h), area(s.y_m, reach, h));
        (y0..=y1)
            .flat_map(|ay| (x0..=x1).map(move |ax| (ax, ay)))
            .collect()
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
            let (s, _) = self.files.settlement(id)?;
            return Ok((
                town::plan_specs(plan, s.population, s.wealth),
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
        Ok(self.generator(id)?.population()?)
    }

    /// The settlement's population skeleton, built once and kept (the
    /// [`GENERATOR_CACHE`] most recently used), so looking up one commoner
    /// does not rebuild the whole settlement each time (review 3.3-10).
    ///
    /// # Errors
    /// As [`World::population`].
    pub fn generator(&self, id: u64) -> Result<Arc<Generator<'static>>, PeopleError> {
        let lock = || {
            self.generators
                .lock()
                .map_err(|_| PeopleError::World("generator cache poisoned".into()))
        };
        {
            let mut g = lock()?;
            g.0 += 1;
            let tick = g.0;
            if let Some(hit) = g.1.get_mut(&id) {
                hit.0 = tick;
                return Ok(Arc::clone(&hit.1));
            }
        }
        let profile = self.profile(id)?;
        let (buildings, _) = self.buildings(id)?;
        let slots = self.slots(id)?;
        self.skeletons_built.fetch_add(1, Ordering::Relaxed);
        let built = Arc::new(
            Generator::with_notables(self.seed(), &profile, &buildings, &slots)?.into_owned(),
        );
        let mut g = lock()?;
        g.0 += 1;
        let tick = g.0;
        g.1.insert(id, (tick, Arc::clone(&built)));
        while g.1.len() > GENERATOR_CACHE {
            let oldest = g.1.iter().min_by_key(|(_, e)| e.0).map(|(k, _)| *k);
            if let Some(k) = oldest {
                g.1.remove(&k);
            }
        }
        Ok(built)
    }

    /// How many population skeletons [`World::generator`] has built (its
    /// cache misses), so callers can check they reuse the cache.
    #[must_use]
    pub fn skeletons_built(&self) -> u64 {
        self.skeletons_built.load(Ordering::Relaxed)
    }

    /// Compares a plan drawn here with the plan the settlement's stored
    /// notables were generated in, by fingerprint; a mismatch marks the
    /// settlement's notables stale (review 3.3-5).
    fn check_plan(&self, id: u64, plan: Option<&TownPlan>) -> Result<(), PeopleError> {
        let Some(entry) = self.notables.as_ref().and_then(|n| n.entry(id)) else {
            return Ok(());
        };
        let (s, _) = self.files.settlement(id)?;
        let here = plan.map(|p| town::fingerprint(&town::plan_specs(p, s.population, s.wealth)));
        if here != entry.plan_fingerprint {
            self.stale_plans
                .lock()
                .map_err(|_| PeopleError::World("stale-plan set poisoned".into()))?
                .insert(id);
        }
        Ok(())
    }

    fn fresh(&self, id: u64) -> Result<(), PeopleError> {
        let stale = self
            .stale_plans
            .lock()
            .map_err(|_| PeopleError::World("stale-plan set poisoned".into()))?
            .contains(&id);
        if stale {
            return Err(PeopleError::Stale(format!(
                "the stored notables of settlement {id} were generated in another town plan \
                 than this build draws: run `arda society build`"
            )));
        }
        Ok(())
    }

    /// A stored notable by id (`notables.json`); `None` when there is none,
    /// its line cannot be read or its plan is stale.
    #[must_use]
    pub fn notable(&self, id: NpcId) -> Option<(&Npc, arda_ids::SettlementId)> {
        self.try_notable(id).ok().flatten()
    }

    /// [`World::notable`] with its failures.
    ///
    /// # Errors
    /// I/O or format errors reading `notables.json`; a stale plan.
    pub fn try_notable(
        &self,
        id: NpcId,
    ) -> Result<Option<(&Npc, arda_ids::SettlementId)>, PeopleError> {
        let Some(n) = self.notables.as_ref() else {
            return Ok(None);
        };
        let found = n.notable(id)?;
        if let Some((_, sid)) = found {
            self.fresh(sid.get())?;
        }
        Ok(found)
    }

    /// Every stored notable of a settlement; empty when there are none, the
    /// file cannot be read or the plan is stale ([`World::try_notables_of`]
    /// says why).
    #[must_use]
    pub fn notables_of(&self, id: u64) -> &[Npc] {
        self.try_notables_of(id).unwrap_or(&[])
    }

    /// Every stored notable of a settlement.
    ///
    /// # Errors
    /// I/O or format errors reading `notables.json`; a stale plan (review
    /// 3.3-5).
    pub fn try_notables_of(&self, id: u64) -> Result<&[Npc], PeopleError> {
        let Some(n) = self.notables.as_ref() else {
            return Ok(&[]);
        };
        self.fresh(id)?;
        Ok(n.of(id)?.map_or(&[], |s| s.npcs.as_slice()))
    }
}

/// Refuses `society/` files built from other inputs than the files beside
/// them or the world they sit in (review 3.3-4): the settlement files must
/// come from this world's `world.json`, and `society.json` and
/// `notables.json` from these settlement files. Files written before
/// stamps existed are accepted only when the settlement files carry no
/// stamp either.
///
/// # Errors
/// [`PeopleError::Stale`] naming the file and the command that rebuilds it.
pub fn check_stamps(
    society_dir: &Path,
    files: &SocietyFiles,
    society: Option<&Society>,
    notables: Option<&Notables>,
) -> Result<(), PeopleError> {
    let world_dir = society_dir.parent().unwrap_or(society_dir);
    let Some(stamp) = files.settlements.stamp.as_ref() else {
        return match (
            society.and_then(|s| s.settle_stamp.as_ref()),
            notables.and_then(|n| n.settle_stamp.as_ref()),
        ) {
            (None, None) => Ok(()),
            _ => Err(PeopleError::Stale(format!(
                "{} has no generation stamp but the society files were built from a stamped \
                 one: run `arda settle` and `arda society build` again",
                society_dir.join("settlements.json").display()
            ))),
        };
    };
    if stamp.world_json != arda_settle::stamp::DETACHED {
        let here = arda_settle::stamp::world_digest(world_dir)
            .map_err(|e| PeopleError::io(&world_dir.join("world.json"), e))?;
        if here != stamp.world_json {
            return Err(PeopleError::Stale(format!(
                "{} was settled from another world.json (stamp {}, world {here}): run `arda \
                 settle --world {}` and `arda society build --world {}`",
                society_dir.display(),
                stamp.world_json,
                world_dir.display(),
                world_dir.display()
            )));
        }
    }
    let built = |name: &str, theirs: Option<&String>| {
        if theirs == Some(&stamp.id) {
            Ok(())
        } else {
            Err(PeopleError::Stale(format!(
                "{} was built from other settlement files (built from {}, settlements {}): run \
                 `arda society build --world {}`",
                society_dir.join(name).display(),
                theirs.map_or("an unstamped run", String::as_str),
                stamp.id,
                world_dir.display()
            )))
        }
    };
    if let Some(s) = society {
        built(SOCIETY_FILE, s.settle_stamp.as_ref())?;
    }
    if let Some(n) = notables {
        built(NOTABLES_FILE, n.settle_stamp.as_ref())?;
    }
    Ok(())
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
