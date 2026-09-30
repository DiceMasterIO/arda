//! Job assignment (`logic/06` step 5 and the spec's "jobs attach to
//! buildings" rule): workplace slots first, residents of a workplace before
//! outsiders, then officials, then land jobs; finally notables are settled
//! into the tier's range.

use crate::error::NpcError;
use crate::input::{BuildingFunction, SettlementFunction};
use crate::plan::{Plan, Role};
use crate::rng::Rng;

/// Chance (per mille) that a family member takes the head's land job.
const SHARE_LAND_JOB_PER_MILLE: u32 = 600;

/// Where officials keep office, in order of preference.
const OFFICE: [BuildingFunction; 5] = [
    BuildingFunction::MarketHall,
    BuildingFunction::Keep,
    BuildingFunction::Manor,
    BuildingFunction::Guardhouse,
    BuildingFunction::Temple,
];

impl Plan<'_> {
    /// Whether someone is of working age for their ancestry.
    pub(crate) fn working(&self, person: usize) -> bool {
        let race = self.race(person);
        let age = self.people[person].age;
        age >= race.working_age && age < race.elder_age
    }

    /// Whether someone is seasoned enough to be a master or official: past
    /// adulthood by a fifth of their working life.
    pub(super) fn mature(&self, person: usize) -> bool {
        let race = self.race(person);
        let seasoned = race.adult_age + (race.elder_age - race.adult_age) / 5;
        self.working(person) && self.people[person].age >= seasoned
    }

    pub(super) fn assign_jobs(&mut self) -> Result<(), NpcError> {
        for p in 0..self.people.len() {
            let race = self.race(p);
            self.people[p].job = if self.people[p].age < race.working_age {
                "child"
            } else if self.people[p].age >= race.elder_age {
                "retired"
            } else {
                ""
            };
        }
        self.slots = self
            .buildings
            .iter()
            .map(|b| vec![None; usize::from(b.workplace_slots)])
            .collect();
        self.staff_from_residents();
        let pool = self.pool();
        let mut taken = vec![false; self.people.len()];
        let mut cursor_any = 0;
        let mut cursor_mature = 0;
        // Two cursors over one seeded order: masters and officials need a
        // mature person, other slots take whoever comes next.
        let mut next = |plan: &Plan<'_>, mature: bool, taken: &mut Vec<bool>| -> Option<usize> {
            let cursor = if mature {
                &mut cursor_mature
            } else {
                &mut cursor_any
            };
            while let Some(&p) = pool.get(*cursor) {
                *cursor += 1;
                if !taken[p] && (!mature || plan.mature(p)) {
                    taken[p] = true;
                    return Some(p);
                }
            }
            None
        };
        for b in 0..self.buildings.len() {
            for s in 0..self.slots[b].len() {
                if self.slots[b][s].is_some() {
                    continue;
                }
                let Some(p) = next(self, s == 0, &mut taken) else {
                    continue;
                };
                self.set_slot(b, s, p);
            }
        }
        let office = OFFICE
            .iter()
            .find_map(|&f| self.buildings.iter().position(|b| b.function == f));
        let mut officials: Vec<&'static str> =
            self.tier.officials.iter().map(String::as_str).collect();
        if self.settlement.has(SettlementFunction::Capital) {
            officials.extend(self.data.tiers.capital_officials.iter().map(String::as_str));
        }
        for job in officials {
            let Some(p) = next(self, true, &mut taken) else {
                break;
            };
            let person = &mut self.people[p];
            person.job = job;
            person.workplace = office;
            person.notable = true;
        }
        self.assign_land_jobs();
        self.settle_notables()
    }

    fn set_slot(&mut self, building: usize, slot: usize, person: usize) {
        let Some(place) = self.workplace(building) else {
            return;
        };
        let job = if slot == 0 {
            place.master.as_deref()
        } else {
            place
                .workers
                .get((slot - 1) % place.workers.len().max(1))
                .map(String::as_str)
        };
        let Some(job) = job else { return };
        self.slots[building][slot] = Some(person);
        let p = &mut self.people[person];
        p.job = job;
        p.workplace = Some(building);
        p.slot = Some(slot);
        p.notable = slot == 0 && place.notable;
    }

    /// Residents staff their own building: the first mature resident is the
    /// master, the master's household takes the building's family job if it
    /// has one, and other residents fill the remaining slots.
    fn staff_from_residents(&mut self) {
        for b in 0..self.buildings.len() {
            let Some(place) = self.workplace(b) else {
                continue;
            };
            if self.slots[b].is_empty() || place.master.is_none() {
                continue;
            }
            let residents: Vec<usize> = (self.offsets[b]..self.offsets[b + 1])
                .filter(|&p| self.working(p) && self.people[p].role != Role::Lodger)
                .collect();
            let Some(&master) = residents.iter().find(|&&p| self.mature(p)) else {
                continue;
            };
            self.set_slot(b, 0, master);
            let master_house = self.people[master].household;
            let mut slot = 1;
            for &p in residents.iter().filter(|&&p| p != master) {
                if let (Some(job), true) = (
                    place.family_job.as_deref(),
                    self.people[p].household == master_house,
                ) {
                    self.people[p].job = job;
                    self.people[p].workplace = Some(b);
                } else if slot < self.slots[b].len() {
                    self.set_slot(b, slot, p);
                    slot += 1;
                }
            }
        }
    }

    /// Unassigned working-age people in a seeded order.
    fn pool(&self) -> Vec<usize> {
        let mut pool: Vec<(u64, usize)> = (0..self.people.len())
            .filter(|&p| self.people[p].job.is_empty())
            .map(|p| (self.person_key(p).hash("pool"), p))
            .collect();
        pool.sort_unstable();
        pool.into_iter().map(|(_, p)| p).collect()
    }

    /// Land-job weights for this settlement: base plus function bonuses,
    /// scaled by the tier's rural or urban factor.
    fn land_weights(&self) -> Vec<(&'static str, u32)> {
        let water = self.settlement.coastal
            || self.settlement.riverine
            || self.settlement.has(SettlementFunction::Fishing)
            || self.settlement.has(SettlementFunction::Port);
        self.data
            .occupations
            .land_jobs
            .iter()
            .filter(|l| l.needs.as_deref() != Some("water") || water)
            .map(|l| {
                let bonus: u32 = self
                    .settlement
                    .functions
                    .iter()
                    .filter_map(|f| l.functions.get(f.key()))
                    .sum();
                let scale = match l.scale.as_str() {
                    "rural" => self.tier.rural_pm,
                    "urban" => self.tier.urban_pm,
                    _ => 1000,
                };
                (l.job.as_str(), (l.base + bonus) * scale / 10)
            })
            .collect()
    }

    fn assign_land_jobs(&mut self) {
        let weights = self.land_weights();
        let w: Vec<u32> = weights.iter().map(|&(_, w)| w).collect();
        for p in 0..self.people.len() {
            if !self.people[p].job.is_empty() {
                continue;
            }
            let mut rng: Rng = self.person_key(p).rng("land-job");
            let house = &self.houses[self.people[p].household];
            let head_job = house
                .members
                .first()
                .filter(|&&h| h != p)
                .map(|&h| self.people[h].job);
            let shares = matches!(
                self.people[p].role,
                Role::Spouse | Role::Child | Role::Sibling
            );
            let job = match head_job {
                Some(j)
                    if shares
                        && weights.iter().any(|&(k, _)| k == j)
                        && rng.chance(SHARE_LAND_JOB_PER_MILLE) =>
                {
                    j
                }
                _ => rng
                    .weighted(&w)
                    .and_then(|i| weights.get(i))
                    .map_or("labourer", |&(k, _)| k),
            };
            let workplace = self.land_workplace(p, job, &mut rng);
            let person = &mut self.people[p];
            person.job = job;
            person.workplace = workplace;
        }
    }

    /// The building a land job is done at, when there is one.
    fn land_workplace(&self, person: usize, job: &str, rng: &mut Rng) -> Option<usize> {
        use BuildingFunction as F;
        let home = self.people[person].building;
        let home_fn = self.buildings[home].function;
        let wanted: &[F] = match job {
            "farmer" | "farmhand" | "herder" | "shepherd"
                if matches!(home_fn, F::Farmhouse | F::Cottage) =>
            {
                return Some(home)
            }
            "farmer" | "farmhand" | "herder" | "shepherd" => &[F::Farmhouse],
            "fisher" => &[F::Boathouse, F::Dock],
            "dockhand" | "sailor" => &[F::Dock],
            "porter" => &[F::Warehouse, F::Dock, F::MarketHall],
            "miner" => &[F::Mine],
            "woodcutter" | "charcoal_burner" => &[F::LumberCamp],
            "carter" => &[F::Stable, F::Warehouse],
            "soldier" => &[F::Barracks, F::Keep],
            "servant" => &[F::Manor, F::Keep, F::Inn],
            "cottage_weaver" | "thatcher" | "midwife" | "chandler" | "launderer" => {
                return Some(home)
            }
            _ => &[],
        };
        let options = self.buildings_with(|f| wanted.contains(&f));
        rng.pick(&options).copied()
    }

    /// Notables are the masters of notable workplaces plus officials. Too
    /// many: the least important are demoted to their job's stat block. Too
    /// few: mature household heads are promoted to local leaders.
    fn settle_notables(&mut self) -> Result<(), NpcError> {
        let [min, max] = self
            .tier
            .notables
            .map(|n| usize::try_from(n).unwrap_or(usize::MAX));
        let mut notables: Vec<usize> = (0..self.people.len())
            .filter(|&p| self.people[p].notable)
            .collect();
        if notables.len() > max {
            let rank = |p: usize| {
                let person = &self.people[p];
                let job = self.data.job(person.job).map(|j| j.leader).unwrap_or(false);
                let (importance, wealth) = person.workplace.map_or((0, 0), |b| {
                    (
                        self.workplace(b).map_or(0, |w| w.importance),
                        self.buildings[b].wealth,
                    )
                });
                (std::cmp::Reverse((job, importance, wealth)), p)
            };
            notables.sort_by_key(|&p| rank(p));
            for &p in &notables[max..] {
                self.people[p].notable = false;
            }
        } else if notables.len() < min {
            let mut heads: Vec<(u64, usize)> = (0..self.people.len())
                .filter(|&p| {
                    let person = &self.people[p];
                    !person.notable
                        && person.slot.is_none()
                        && person.role == Role::Head
                        && self.mature(p)
                })
                .map(|p| (self.person_key(p).hash("promote"), p))
                .collect();
            heads.sort_unstable();
            let titles = &self.tier.promote;
            for (i, &(_, p)) in heads.iter().take(min - notables.len()).enumerate() {
                let Some(job) = titles.get(i % titles.len().max(1)) else {
                    break;
                };
                let person = &mut self.people[p];
                person.job = job.as_str();
                person.workplace = Some(person.building);
                person.notable = true;
            }
        }
        Ok(())
    }
}
