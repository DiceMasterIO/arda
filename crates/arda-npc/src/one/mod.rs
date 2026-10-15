//! One NPC from a few optional inputs, without a settlement profile or a
//! building list, and without storing anything (logic/13
//! §npc-generate-one; `POST /v1/npc/generate`).
//!
//! The generator builds a small synthetic settlement with the reserved id
//! [`SYNTHETIC_SETTLEMENT`] (0, never a real settlement: world settlement
//! ids start at 1), laid out so the requested job exists in it, runs the
//! ordinary population skeleton over it and returns one inhabitant who
//! matches every filter. Nothing about the synthetic settlement points
//! into a world: relationships, employer and workplace are cleared, and
//! the id is a keyed hash of the request that no route resolves.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::error::NpcError;
use crate::input::{
    Biome, NotableSlot, RealmId, SettlementFunction, SettlementId, SettlementProfile, Tier,
};
use crate::npc::{Npc, NpcId, Sex, SocialRank};
use crate::plan::Plan;

mod layout;

use layout::{buildings, notable_source, office_building, office_slot, reachable, route, Route};

/// Settlement id of the synthetic settlement: "not in the world".
pub const SYNTHETIC_SETTLEMENT: SettlementId = SettlementId(0);
/// Inhabitants of the synthetic settlement; a request costs milliseconds.
pub const PEOPLE: u32 = 40;
/// Most synthetic settlements tried before a request is unsatisfiable.
pub const MAX_ATTEMPTS: u64 = 16;
/// Copies of the workplace that staffs the requested job.
const WORKPLACE_COPIES: usize = 2;
/// Wealth of the synthetic settlement when none is borrowed.
const DEFAULT_WEALTH: u8 = 120;
/// Residents of one synthetic dwelling (a sample `house`).
const HOUSE_CAPACITY: u16 = 6;
/// Tiers, smallest first.
const TIERS: [Tier; 4] = [Tier::Hamlet, Tier::Village, Tier::Town, Tier::City];

/// What a caller asks of [`generate_one`]. Every field is optional; an
/// empty request is a working adult of a village.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneRequest {
    /// Caller's salt; the same salt and inputs give the same person.
    #[serde(default)]
    pub seed: u64,
    /// Occupation key (`occupations.json` `jobs`), e.g. `smith`.
    #[serde(default)]
    pub job: Option<String>,
    /// Society office kind (`occupations.json` `offices`), e.g. `reeve`;
    /// the person holds it and is a stored notable.
    #[serde(default)]
    pub office: Option<String>,
    /// Social rank the person must have.
    #[serde(default)]
    pub rank: Option<SocialRank>,
    /// SRD race key, e.g. `dwarf`.
    #[serde(default)]
    pub ancestry: Option<String>,
    /// Sex the person must have.
    #[serde(default)]
    pub sex: Option<Sex>,
    /// Size class; overrides the borrowed settlement's. Default: the
    /// borrowed settlement's, else `village` when the job exists there,
    /// else the smallest tier that has the job.
    #[serde(default)]
    pub tier: Option<Tier>,
    /// Culture key (`cultures.json`); overrides the borrowed settlement's
    /// culture and its ancestry mix.
    #[serde(default)]
    pub culture: Option<String>,
    /// A real settlement whose tier, functions, wealth, culture, biome,
    /// tongue, ancestry mix and name the synthetic settlement borrows.
    #[serde(default)]
    pub settlement: Option<SettlementProfile>,
    /// Whether the person must (or must not) be a stored notable.
    #[serde(default)]
    pub notable: Option<bool>,
}

/// Generates one NPC for `request` in the world of `world_seed`.
///
/// # Errors
/// [`NpcError::UnknownKey`] for a key the tables lack,
/// [`NpcError::Unreachable`] for inputs that rule each other out (a rank
/// the job lacks in the tier, a job the tier lacks),
/// [`NpcError::Unsatisfiable`] when no bounded attempt yields a match, and
/// [`NpcError::Data`] for invalid bundled data.
pub fn generate_one(world_seed: u64, request: &OneRequest) -> Result<Npc, NpcError> {
    let data = Data::get()?;
    let job = request
        .job
        .as_deref()
        .map(|k| known_job(data, k))
        .transpose()?;
    let office = request
        .office
        .as_deref()
        .map(|k| known_office(data, k))
        .transpose()?;
    let profile = profile(data, request)?;
    let tier = tier(data, request, job)?;
    if let (Some(job), Some(rank)) = (job, request.rank) {
        check_rank(data, job, office, tier, rank)?;
    }
    let candidates = if job.is_none() && office.is_none() {
        candidates(data, request, tier)?
    } else {
        Vec::new()
    };
    let mut profile = profile;
    profile.tier = tier;
    if request.settlement.is_none() {
        profile.name = format!("the {}", tier.key());
    }
    for attempt in 0..MAX_ATTEMPTS {
        let seed = arda_ids::subseed(world_seed, "npc-generate", &[request.seed, attempt]);
        let target = job.or_else(|| {
            let pick = arda_ids::subseed(seed, "npc-generate/job", &[]);
            let len = u64::try_from(candidates.len()).ok()?;
            let i = usize::try_from(pick % len.max(1)).ok()?;
            candidates.get(i).copied()
        });
        let route = match (target, office) {
            (Some(j), _) => route(data, j, tier),
            (None, Some(o)) => office_building(o).map_or(Route::Houses, Route::Workplace),
            (None, None) => Route::Sample,
        };
        let mut place = profile.clone();
        if target.is_some_and(|j| data.tiers.capital_officials.iter().any(|c| c == j))
            && !place.has(SettlementFunction::Capital)
        {
            place.functions.push(SettlementFunction::Capital);
        }
        let buildings = buildings(data, route, &place, seed);
        let slots: Vec<NotableSlot> = office
            .map(|kind| office_slot(data, kind, route, &buildings, request.sex))
            .into_iter()
            .collect();
        let land = match route {
            Route::Land(j) => Some(j),
            _ => None,
        };
        let plan = Plan::build_with(seed, &place, &buildings, &slots, land)?;
        let matches: Vec<usize> = (0..plan.people.len())
            .filter(|&p| fits(&plan, p, request, target, office.is_some()))
            .collect();
        let len = u64::try_from(matches.len()).unwrap_or(u64::MAX);
        if len == 0 {
            continue;
        }
        let pick = arda_ids::subseed(seed, "npc-generate/pick", &[]) % len;
        let person = matches[usize::try_from(pick).unwrap_or(0)];
        let mut npc = plan.npc(person)?;
        npc.id = NpcId(arda_ids::subseed(seed, "npc-generate/id", &[npc.id.0]));
        npc.relationships.clear();
        npc.employer = None;
        npc.workplace_building = None;
        return Ok(npc);
    }
    Err(NpcError::Unsatisfiable(format!(
        "no inhabitant of {MAX_ATTEMPTS} synthetic {}s matched every filter",
        tier.key()
    )))
}

/// Whether skeleton person `p` passes every filter of the request.
fn fits(plan: &Plan<'_>, p: usize, request: &OneRequest, job: Option<&str>, office: bool) -> bool {
    let person = &plan.people[p];
    let asked_job = job.is_some() || request.rank.is_some();
    job.is_none_or(|j| person.job == j)
        && (asked_job || person.job != "child")
        && (!office || plan.slot_of(p, &plan.notables).is_some())
        && request
            .rank
            .is_none_or(|r| SocialRank::from_level(plan.rank_of(p)) == r)
        && request.sex.is_none_or(|s| person.sex == s)
        && request.notable.is_none_or(|n| person.notable == n)
        && request
            .ancestry
            .as_deref()
            .is_none_or(|a| plan.race(p).key == a)
}

/// The synthetic settlement's profile: borrowed context, overridden by
/// the request's culture and ancestry.
fn profile(data: &'static Data, request: &OneRequest) -> Result<SettlementProfile, NpcError> {
    let mut profile = request
        .settlement
        .clone()
        .unwrap_or_else(|| SettlementProfile {
            id: SYNTHETIC_SETTLEMENT,
            name: String::new(),
            tier: Tier::Village,
            population: PEOPLE,
            functions: vec![SettlementFunction::Farming],
            wealth: DEFAULT_WEALTH,
            culture: data.cultures.default.clone(),
            realm_id: RealmId(0),
            biome: Biome::Temperate,
            coastal: false,
            riverine: false,
            ancestry_mix: None,
            tongue: None,
        });
    profile.id = SYNTHETIC_SETTLEMENT;
    profile.population = PEOPLE;
    if let Some(culture) = &request.culture {
        if !data.cultures.cultures.contains_key(culture) {
            return Err(unknown("culture", culture, data.cultures.cultures.keys()));
        }
        culture.clone_into(&mut profile.culture);
        profile.ancestry_mix = None;
    }
    if let Some(ancestry) = &request.ancestry {
        if !data.srd.races.iter().any(|r| r.key == *ancestry) {
            return Err(unknown(
                "ancestry",
                ancestry,
                data.srd.races.iter().map(|r| &r.key),
            ));
        }
        profile.ancestry_mix = Some(BTreeMap::from([(ancestry.clone(), 1)]));
    }
    Ok(profile)
}

fn unknown<'k>(what: &'static str, key: &str, valid: impl Iterator<Item = &'k String>) -> NpcError {
    NpcError::UnknownKey {
        what,
        key: key.to_owned(),
        valid: valid.map(String::as_str).collect::<Vec<_>>().join(", "),
    }
}

fn known_job(data: &'static Data, key: &str) -> Result<&'static str, NpcError> {
    data.occupations
        .jobs
        .get_key_value(key)
        .map(|(k, _)| k.as_str())
        .ok_or_else(|| unknown("job", key, data.occupations.jobs.keys()))
}

fn known_office(data: &'static Data, key: &str) -> Result<&'static str, NpcError> {
    data.occupations
        .offices
        .get_key_value(key)
        .map(|(k, _)| k.as_str())
        .ok_or_else(|| unknown("office", key, data.occupations.offices.keys()))
}

/// The tier: the request's, the borrowed settlement's, else village when
/// the job exists there, else the smallest tier that has it.
fn tier(data: &'static Data, request: &OneRequest, job: Option<&str>) -> Result<Tier, NpcError> {
    let fixed = request
        .tier
        .or_else(|| request.settlement.as_ref().map(|s| s.tier));
    let Some(job) = job else {
        return Ok(fixed.unwrap_or(Tier::Village));
    };
    let tiers: Vec<Tier> = TIERS
        .into_iter()
        .filter(|&t| reachable(data, job, t))
        .collect();
    let chosen = match fixed {
        Some(t) => tiers.contains(&t).then_some(t),
        None if tiers.contains(&Tier::Village) => Some(Tier::Village),
        None => tiers.first().copied(),
    };
    chosen.ok_or_else(|| {
        let valid: Vec<&str> = tiers.iter().map(|t| t.key()).collect();
        NpcError::Unreachable(format!(
            "no {} has a {job}; valid: {}",
            fixed.map_or("tier", Tier::key),
            valid.join(", ")
        ))
    })
}

fn rank_key(rank: SocialRank) -> String {
    format!("{rank:?}").to_lowercase()
}

/// Refuses a rank the job (raised by the office) does not have in the tier.
fn check_rank(
    data: &'static Data,
    job: &str,
    office: Option<&str>,
    tier: Tier,
    rank: SocialRank,
) -> Result<(), NpcError> {
    let job_level = data.job(job)?.rank_in(tier.key());
    let office_level = office
        .and_then(|o| data.occupations.offices.get(o))
        .map_or(0, |o| o.rank_in(tier.key()));
    let has = SocialRank::from_level(job_level.max(office_level));
    if has == rank {
        return Ok(());
    }
    Err(NpcError::Unreachable(format!(
        "a {job} ranks {} in a {}, not {}; valid: {}",
        rank_key(has),
        tier.key(),
        rank_key(rank),
        rank_key(has)
    )))
}

/// Jobs a rank- or notable-only request picks from, in key order.
fn candidates(
    data: &'static Data,
    request: &OneRequest,
    tier: Tier,
) -> Result<Vec<&'static str>, NpcError> {
    if request.rank.is_none() && request.notable.is_none() {
        return Ok(Vec::new());
    }
    let open: Vec<&'static str> = data
        .occupations
        .jobs
        .keys()
        .map(String::as_str)
        .filter(|&j| reachable(data, j, tier))
        .filter(|&j| {
            request
                .notable
                .is_none_or(|n| notable_source(data, j, tier) == n)
        })
        .collect();
    let rank_of = |j: &str| {
        data.job(j)
            .map(|d| SocialRank::from_level(d.rank_in(tier.key())))
    };
    let mut out = Vec::new();
    for &j in &open {
        let rank = rank_of(j)?;
        let wanted = match request.rank {
            Some(r) => rank == r,
            None => j != "child",
        };
        if wanted {
            out.push(j);
        }
    }
    if out.is_empty() {
        let mut valid: Vec<SocialRank> = open.iter().filter_map(|&j| rank_of(j).ok()).collect();
        valid.sort_unstable();
        valid.dedup();
        let valid: Vec<String> = valid.into_iter().map(rank_key).collect();
        return Err(NpcError::Unreachable(format!(
            "no job reaches rank {} in a {}; valid: {}",
            request.rank.map_or_else(|| "any".to_owned(), rank_key),
            tier.key(),
            valid.join(", ")
        )));
    }
    Ok(out)
}
