//! Households and their members, generated per building from the
//! building's own seed key.
//!
//! Ages are in the ancestry's own years. A parent is always at least
//! `gap_min = max(16, adult_age * 3 / 4)` years older than a child, and
//! nobody exceeds the ancestry's maximum age.

use crate::data::content::HouseholdStyle;
use crate::data::srd::RaceData;
use crate::error::NpcError;
use crate::npc::{RelationKind, Sex};
use crate::plan::{House, Person, Plan, Role};
use crate::rng::Rng;

/// Per mille chances used when composing a family.
const SPOUSE_PER_MILLE: u32 = 850;
const SAME_ANCESTRY_SPOUSE_PER_MILLE: u32 = 900;
const ELDER_PARENT_PER_MILLE: u32 = 120;
const SIBLING_PER_MILLE: u32 = 100;
const LODGER_PER_MILLE: u32 = 600;

/// Minimum parent–child age gap for an ancestry.
#[must_use]
pub(crate) fn gap_min(race: &RaceData) -> u16 {
    (race.adult_age * 3 / 4).max(16)
}

fn gap_max(race: &RaceData) -> u16 {
    gap_min(race) + (race.max_age - race.adult_age) / 3
}

fn age_between(rng: &mut Rng, lo: u16, hi: u16) -> u16 {
    u16::try_from(rng.range(u32::from(lo), u32::from(hi.max(lo)))).unwrap_or(lo)
}

/// Mean of two draws: ages cluster in the middle of the range.
fn age_middle(rng: &mut Rng, lo: u16, hi: u16) -> u16 {
    let a = age_between(rng, lo, hi);
    let b = age_between(rng, lo, hi);
    a / 2 + b / 2 + (a % 2 + b % 2) / 2
}

/// Minimum of two draws: children skew young.
fn age_young(rng: &mut Rng, lo: u16, hi: u16) -> u16 {
    age_between(rng, lo, hi).min(age_between(rng, lo, hi))
}

impl Plan<'_> {
    fn pick_race(&self, rng: &mut Rng) -> usize {
        let weights: Vec<u32> = self.ancestry.iter().map(|&(_, w)| w).collect();
        rng.weighted(&weights)
            .and_then(|i| self.ancestry.get(i))
            .map_or(0, |&(r, _)| r)
    }

    fn race_key(&self, race: usize) -> &str {
        self.data
            .srd
            .races
            .get(race)
            .map_or("human", |r| r.key.as_str())
    }

    fn race_index(&self, key: &str) -> usize {
        self.data
            .srd
            .races
            .iter()
            .position(|r| r.key == key)
            .unwrap_or(0)
    }

    /// Child of two ancestries: human with elf gives a half-elf, a half-orc
    /// parent gives a half-orc, otherwise one parent's ancestry.
    fn child_race(&self, a: usize, b: usize, rng: &mut Rng) -> usize {
        let (ka, kb) = (self.race_key(a), self.race_key(b));
        let pair = |x: &str, y: &str| (ka == x && kb == y) || (ka == y && kb == x);
        if a == b {
            a
        } else if pair("human", "elf") || pair("half-elf", "elf") || pair("half-elf", "human") {
            self.race_index("half-elf")
        } else if ka == "half-orc" || kb == "half-orc" {
            self.race_index("half-orc")
        } else if rng.chance(500) {
            a
        } else {
            b
        }
    }

    /// Creates every building's people and households.
    pub(super) fn populate(&mut self, occupancy: &[u32]) -> Result<(), NpcError> {
        let total: usize = occupancy
            .iter()
            .map(|&o| usize::try_from(o).unwrap_or(0))
            .sum();
        self.people.reserve_exact(total);
        self.offsets.push(0);
        for (b, &occ) in occupancy.iter().enumerate() {
            let style = self
                .workplace(b)
                .map_or(HouseholdStyle::Family, |w| w.household);
            let mut rng = self.building_key(b).rng("households");
            let sizes = self.household_sizes(style, occ, &mut rng);
            for (h, &(size, lodger)) in sizes.iter().enumerate() {
                let index =
                    u32::try_from(h).map_err(|_| NpcError::Data("too many households".into()))?;
                let singles = style == HouseholdStyle::Singles
                    || (style == HouseholdStyle::FamilyWithStaff && h > 0);
                self.compose(b, index, size, singles, lodger, &mut rng);
            }
            self.offsets.push(self.people.len());
        }
        Ok(())
    }

    /// Household sizes and whether each takes a lodger, summing to `occupancy`.
    fn household_sizes(
        &self,
        style: HouseholdStyle,
        occupancy: u32,
        rng: &mut Rng,
    ) -> Vec<(u32, bool)> {
        let weights: Vec<u32> = self
            .data
            .tiers
            .household_sizes
            .iter()
            .map(|&[_, w]| w)
            .collect();
        let draw = |rng: &mut Rng| -> u32 {
            rng.weighted(&weights)
                .and_then(|i| self.data.tiers.household_sizes.get(i))
                .map_or(4, |&[s, _]| s)
        };
        let mut sizes = Vec::new();
        let mut left = occupancy;
        if style == HouseholdStyle::FamilyWithStaff && left > 0 {
            let s = rng.range(2, 5).min(left);
            sizes.push((s, false));
            left -= s;
        }
        while left > 0 {
            let s = if style == HouseholdStyle::Family {
                draw(rng).min(left)
            } else {
                1
            };
            sizes.push((s, false));
            left -= s;
        }
        if style == HouseholdStyle::Family
            && sizes.len() > 1
            && sizes.last().is_some_and(|&(s, _)| s == 1)
            && rng.chance(LODGER_PER_MILLE)
        {
            sizes.pop();
            if let Some(last) = sizes.last_mut() {
                *last = (last.0, true);
            }
        }
        sizes
    }

    fn push_person(
        &mut self,
        building: usize,
        household: usize,
        role: Role,
        race: usize,
        sex: Sex,
        age: u16,
    ) -> usize {
        let start = self.offsets.last().copied().unwrap_or(0);
        let index = u32::try_from(self.people.len() - start).unwrap_or(u32::MAX);
        self.people.push(Person {
            building,
            index,
            household,
            role,
            race,
            sex,
            age,
            job: "child",
            workplace: None,
            slot: None,
            notable: false,
        });
        self.people.len() - 1
    }

    fn random_sex(rng: &mut Rng) -> Sex {
        if rng.chance(500) {
            Sex::Female
        } else {
            Sex::Male
        }
    }

    /// Composes one household of `size` (plus an optional lodger).
    fn compose(
        &mut self,
        building: usize,
        index: u32,
        size: u32,
        single: bool,
        lodger: bool,
        rng: &mut Rng,
    ) {
        let house = self.houses.len();
        let mut members = Vec::new();
        let mut rel = Vec::new();
        let head_race = self.pick_race(rng);
        let race = self
            .data
            .srd
            .races
            .get(head_race)
            .unwrap_or(&self.data.srd.races[0]);
        if single || size == 1 {
            let hi = if single {
                race.elder_age - 1
            } else {
                race.elder_age + (race.max_age - race.elder_age) / 4
            };
            let age = age_middle(rng, race.working_age.max(16), hi);
            let role = if single { Role::Single } else { Role::Head };
            members.push(self.push_person(
                building,
                house,
                role,
                head_race,
                Self::random_sex(rng),
                age,
            ));
        } else {
            self.compose_family(
                building,
                house,
                size,
                head_race,
                rng,
                &mut members,
                &mut rel,
            );
        }
        if lodger {
            let lr = self.pick_race(rng);
            let lrace = self
                .data
                .srd
                .races
                .get(lr)
                .unwrap_or(&self.data.srd.races[0]);
            let age = age_between(rng, lrace.working_age.max(16), lrace.elder_age - 1);
            let l = self.push_person(
                building,
                house,
                Role::Lodger,
                lr,
                Self::random_sex(rng),
                age,
            );
            if let Some(&head) = members.first() {
                rel.push((l, RelationKind::Landlord, head));
                rel.push((head, RelationKind::Lodger, l));
            }
            members.push(l);
        }
        self.houses.push(House {
            building,
            index,
            members,
            relations: rel,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn compose_family(
        &mut self,
        building: usize,
        house: usize,
        size: u32,
        head_race: usize,
        rng: &mut Rng,
        members: &mut Vec<usize>,
        rel: &mut Vec<(usize, RelationKind, usize)>,
    ) {
        let races = &self.data.srd.races;
        let race = races.get(head_race).unwrap_or(&races[0]);
        let (gmin, gmax) = (gap_min(race), gap_max(race));
        let has_spouse = rng.chance(SPOUSE_PER_MILLE);
        let mut roles = vec![if has_spouse {
            Role::Spouse
        } else {
            Role::Sibling
        }];
        let mut parent = false;
        for _ in 2..size {
            if !parent && rng.chance(ELDER_PARENT_PER_MILLE) {
                parent = true;
                roles.push(Role::Parent);
            } else if rng.chance(SIBLING_PER_MILLE) {
                roles.push(Role::Sibling);
            } else {
                roles.push(Role::Child);
            }
        }
        let wants_children = roles.contains(&Role::Child);
        let head_lo = if wants_children {
            race.adult_age.max(gmin)
        } else {
            race.adult_age
        };
        let head_hi = if wants_children {
            race.elder_age
        } else {
            race.elder_age + (race.max_age - race.elder_age) / 4
        };
        let head_age = age_middle(rng, head_lo, head_hi);
        // An elder parent shares the head's ancestry; a half-elf's parent is
        // human or elf, and must fit that ancestry's lifespan.
        let parent_race = if self.race_key(head_race) == "half-elf" {
            if rng.chance(500) {
                self.race_index("human")
            } else {
                self.race_index("elf")
            }
        } else {
            head_race
        };
        let parent_max = races.get(parent_race).map_or(race.max_age, |r| r.max_age);
        let parent_ok = head_age + gmin < parent_max;
        let head_sex = Self::random_sex(rng);
        let head = self.push_person(building, house, Role::Head, head_race, head_sex, head_age);
        members.push(head);
        let spread = (race.max_age - race.adult_age) / 12 + 2;
        let mut spouse = None;
        let mut elder = None;
        let mut youngest_parent = head_age;
        for role in roles
            .iter()
            .copied()
            .filter(|r| matches!(r, Role::Spouse | Role::Parent))
        {
            if role == Role::Spouse {
                let bounds = |r: &RaceData| {
                    let lo = head_age
                        .saturating_sub(spread)
                        .max(r.adult_age)
                        .max(if wants_children { gmin } else { 0 });
                    (lo, (head_age + spread).min(r.max_age - 1))
                };
                let mut sr = if rng.chance(SAME_ANCESTRY_SPOUSE_PER_MILLE) {
                    head_race
                } else {
                    self.pick_race(rng)
                };
                let (mut lo, mut hi) = bounds(races.get(sr).unwrap_or(&races[0]));
                if lo > hi {
                    sr = head_race;
                    (lo, hi) = bounds(race);
                }
                let age = age_between(rng, lo, hi);
                youngest_parent = youngest_parent.min(age);
                let sex = if rng.chance(950) {
                    if head_sex == Sex::Female {
                        Sex::Male
                    } else {
                        Sex::Female
                    }
                } else {
                    head_sex
                };
                let s = self.push_person(building, house, Role::Spouse, sr, sex, age);
                rel.push((head, RelationKind::Spouse, s));
                rel.push((s, RelationKind::Spouse, head));
                members.push(s);
                spouse = Some((s, sr));
            } else if parent_ok {
                let age = age_between(rng, head_age + gmin, (head_age + gmax).min(parent_max - 1));
                let p = self.push_person(
                    building,
                    house,
                    Role::Parent,
                    parent_race,
                    Self::random_sex(rng),
                    age,
                );
                rel.push((head, RelationKind::Parent, p));
                rel.push((p, RelationKind::Child, head));
                members.push(p);
                elder = Some((p, age));
            }
        }
        let mut siblings = vec![head];
        let mut children: Vec<usize> = Vec::new();
        // Children still at home are mostly young: cap them a little past
        // adulthood.
        let home_cap = race.adult_age + (race.max_age - race.adult_age) / 10;
        let child_hi = youngest_parent.checked_sub(gmin).map(|hi| hi.min(home_cap));
        let rest = roles
            .iter()
            .copied()
            .filter(|&r| r != Role::Spouse && !(r == Role::Parent && parent_ok));
        for role in rest {
            let as_child = matches!(role, Role::Child | Role::Parent);
            if let (true, Some(hi)) = (as_child, child_hi) {
                let lo = youngest_parent.saturating_sub(gmax).min(hi);
                let age = age_young(rng, lo, hi);
                let cr = spouse.map_or(head_race, |(_, sr)| self.child_race(head_race, sr, rng));
                let c =
                    self.push_person(building, house, Role::Child, cr, Self::random_sex(rng), age);
                for parent_idx in std::iter::once(head).chain(spouse.map(|(s, _)| s)) {
                    rel.push((c, RelationKind::Parent, parent_idx));
                    rel.push((parent_idx, RelationKind::Child, c));
                }
                for &other in &children {
                    rel.push((c, RelationKind::Sibling, other));
                    rel.push((other, RelationKind::Sibling, c));
                }
                children.push(c);
                members.push(c);
            } else {
                let cap = elder.map_or(race.max_age - 1, |(_, pa)| pa.saturating_sub(gmin));
                let lo = head_age
                    .saturating_sub(spread)
                    .max(race.working_age.max(16))
                    .min(cap);
                let age = age_between(rng, lo, (head_age + spread).min(cap).max(lo));
                let s = self.push_person(
                    building,
                    house,
                    Role::Sibling,
                    head_race,
                    Self::random_sex(rng),
                    age,
                );
                if let Some((p, _)) = elder {
                    rel.push((s, RelationKind::Parent, p));
                    rel.push((p, RelationKind::Child, s));
                }
                for &other in &siblings {
                    rel.push((s, RelationKind::Sibling, other));
                    rel.push((other, RelationKind::Sibling, s));
                }
                siblings.push(s);
                members.push(s);
            }
        }
    }
}
