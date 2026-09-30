//! Expands one skeleton person into a full [`Npc`]. Everything drawn here
//! comes from the person's own seed key (or their household's), so the
//! result does not depend on which other people were expanded.

use crate::data::content::JobData;
use crate::error::NpcError;
use crate::input::label;
use crate::npc::{Job, JobCategory, Lifestyle, Name, Npc, RelationKind, SocialRank};
use crate::personality::{generate, BondContext};
use crate::plan::{Plan, Role};
use crate::rng::Rng;
use crate::sheet::{build_block_sheet, build_class_sheet, ClassRequest, Sheet, Wealth};

impl Plan<'_> {
    /// The person's name (cheap; used for bonds that mention others), in
    /// the tongue of their ancestry or settlement (`names.rs`). A slot-bound
    /// notable carries the slot's names; household members inherit the
    /// household's family name, which a slot-bound head fixes.
    pub(crate) fn name_of(&self, person: usize) -> Result<Name, NpcError> {
        let p = &self.people[person];
        let race = self.race(person);
        let lang = self.tongues.of(&race.key);
        let own_family = matches!(p.role, Role::Lodger | Role::Single);
        let slot = self.slot_of(person, self.notables);
        let house = &self.houses[p.household];
        let head = house.members.first().copied().unwrap_or(person);
        let head_family = self
            .slot_of(head, self.notables)
            .and_then(|s| s.family_name.clone());
        let inherited = if own_family {
            None
        } else {
            let head_lang = self.tongues.of(&self.race(head).key);
            let key = self
                .building_key(house.building)
                .with_index(house.index)
                .hash("family-name");
            crate::names::house_family(head_lang, key)
        };
        let adult = p.age >= race.working_age;
        let key = self.person_key(person).hash("name");
        let drawn = crate::names::person(lang, p.sex, key, inherited, adult);
        let family = slot
            .and_then(|s| s.family_name.clone())
            .or(if own_family { None } else { head_family })
            .or_else(|| drawn.family.map(|f| f.native))
            .unwrap_or_default();
        Ok(Name {
            given: slot
                .and_then(|s| s.given_name.clone())
                .unwrap_or(drawn.given.native),
            family,
            byname: drawn.byname.map(|b| b.native),
        })
    }

    /// Display title of a job at a workplace ("Master Weaver", "Priestess").
    fn job_title(&self, person: usize, job: &JobData) -> String {
        let p = &self.people[person];
        let craft = p
            .workplace
            .and_then(|b| self.crafts[b])
            .and_then(|c| self.data.occupations.crafts.get(c.key()));
        match (p.job, craft) {
            ("craft_master", Some(c)) => format!("Master {}", c.noun),
            ("journeyman", Some(c)) => format!("Journeyman {}", c.noun),
            ("apprentice", Some(c)) => format!("{}'s Apprentice", c.noun),
            _ => match (&job.title_female, p.sex) {
                (Some(t), crate::npc::Sex::Female) => t.clone(),
                _ => job.title.clone(),
            },
        }
    }

    /// Wealth 0–255 from settlement, home and rank.
    fn wealth(&self, person: usize, rank: u8, rng: &mut Rng) -> u8 {
        let p = &self.people[person];
        let home = u32::from(self.buildings[p.building].wealth);
        let town = u32::from(self.settlement.wealth);
        let rank = u32::from(rank);
        let value = town / 4 + home / 4 + rank * 30 + rng.below(40);
        u8::try_from(value.min(255)).unwrap_or(u8::MAX)
    }

    /// Class level of a notable: the tier's range (leaders use the leader
    /// range), with a rare jump of a few levels.
    fn notable_level(&self, leader: bool, rng: &mut Rng) -> u8 {
        let [lo, hi] = if leader {
            self.tier.leader_levels
        } else {
            self.tier.levels
        };
        let mut level = u8::try_from(rng.range(u32::from(lo), u32::from(hi))).unwrap_or(lo);
        let exception = &self.data.tiers.exception;
        if rng.chance(exception.chance_per_mille) {
            level = (level + exception.bonus_levels).min(exception.max_level);
        }
        level
    }

    fn sheet(&self, person: usize, job: &JobData, lifestyle: Lifestyle) -> Result<Sheet, NpcError> {
        let p = &self.people[person];
        let race = self.race(person);
        let key = self.person_key(person);
        let craft_tool = p
            .workplace
            .and_then(|b| self.crafts[b])
            .and_then(|c| self.data.occupations.crafts.get(c.key()))
            .map(|c| c.tool.as_str());
        let items = crate::sheet::equipment_for(job, craft_tool, lifestyle);
        let coins = crate::sheet::purse(lifestyle, &mut key.rng("coins"));
        let mut rng = key.rng("sheet");
        if p.notable {
            let classes: Vec<(&String, &u32)> = job.classes.iter().collect();
            let weights: Vec<u32> = classes.iter().map(|&(_, &w)| w).collect();
            let class_key = rng
                .weighted(&weights)
                .and_then(|i| classes.get(i))
                .map_or("fighter", |&(k, _)| k.as_str());
            let class = self
                .data
                .srd
                .class(class_key)
                .ok_or_else(|| NpcError::Data(format!("class {class_key}")))?;
            let background = self
                .data
                .backgrounds
                .backgrounds
                .get(&job.category)
                .ok_or_else(|| NpcError::Data(format!("background {}", job.category)))?;
            let request = ClassRequest {
                class,
                race,
                level: self.notable_level(job.leader, &mut rng),
                background,
                wealth: Wealth::from_lifestyle(lifestyle),
                items: &items,
                coins,
            };
            build_class_sheet(self.data, &request, &mut rng)
        } else {
            let block = self
                .data
                .srd
                .stat_block(&job.stat_block)
                .ok_or_else(|| NpcError::Data(format!("stat block {}", job.stat_block)))?;
            Ok(build_block_sheet(
                self.data, block, race, &items, coins, &mut rng,
            ))
        }
    }

    /// Expands one person.
    pub(crate) fn npc(&self, person: usize) -> Result<Npc, NpcError> {
        let p = &self.people[person];
        let race = self.race(person);
        let job_data = self.data.job(p.job)?;
        let category = JobCategory::from_key(&job_data.category)
            .ok_or_else(|| NpcError::Data(format!("category {}", job_data.category)))?;
        let key = self.person_key(person);
        let rank = job_data.rank;
        let wealth = self.wealth(person, rank, &mut key.rng("wealth"));
        let lifestyle = Lifestyle::from_wealth(wealth);
        let title = self
            .slot_of(person, self.notables)
            .map_or_else(|| self.job_title(person, job_data), |s| s.title.clone());
        let relationships = self.relationships(person);
        let kin = relationships
            .iter()
            .find(|r| {
                matches!(
                    r.kind,
                    RelationKind::Spouse
                        | RelationKind::Child
                        | RelationKind::Parent
                        | RelationKind::Sibling
                )
            })
            .and_then(|r| self.index_of(r.other).ok());
        let given = |q: Option<usize>| -> Result<Option<String>, NpcError> {
            q.map(|q| self.name_of(q).map(|n| n.given)).transpose()
        };
        let full = |q: Option<usize>| -> Result<Option<String>, NpcError> {
            q.map(|q| self.name_of(q).map(|n| n.full())).transpose()
        };
        let workplace_label = p
            .workplace
            .map(|b| format!("the {}", label(self.buildings[b].function, self.crafts[b])));
        let (kin_name, friend_name, rival_name) = (
            given(kin)?,
            full(self.friend[person])?,
            full(self.rival[person])?,
        );
        let ctx = BondContext {
            settlement: &self.settlement.name,
            workplace: workplace_label.as_deref(),
            kin: kin_name.as_deref(),
            friend: friend_name.as_deref(),
            rival: rival_name.as_deref(),
        };
        let personality = generate(
            self.data,
            &job_data.category,
            &title,
            &ctx,
            &mut key.rng("personality"),
        );
        let house = &self.houses[p.household];
        Ok(Npc {
            id: self.id_of(person),
            name: self.name_of(person)?,
            ancestry: race.key.clone(),
            ancestry_name: race.name.clone(),
            subrace: race.subrace.clone(),
            age: p.age,
            sex: p.sex,
            job: Job {
                key: p.job.to_string(),
                title,
                category,
            },
            workplace_building: p.workplace.map(|b| self.buildings[b].id),
            home_building: self.buildings[p.building].id,
            household: crate::npc::HouseholdId {
                building: self.buildings[house.building].id,
                index: house.index,
            },
            employer: self.employer(person).map(|e| self.id_of(e)),
            social_rank: SocialRank::from_level(rank),
            wealth,
            lifestyle,
            personality,
            relationships,
            sheet: self.sheet(person, job_data, lifestyle)?,
            notable: p.notable,
        })
    }
}
