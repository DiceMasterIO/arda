//! The synthetic settlement of [`super::generate_one`]: where each job
//! comes from (a workplace, the land, an official post, a promotion or
//! age) and the buildings that make it exist (logic/13 §npc-generate-one).

use crate::data::Data;
use crate::input::{
    BuildingFunction as F, BuildingId, BuildingSpec, NotableSlot, SettlementProfile, Tier,
};
use crate::npc::Sex;

use super::{HOUSE_CAPACITY, PEOPLE, SYNTHETIC_SETTLEMENT, WORKPLACE_COPIES};

/// How the synthetic settlement is laid out.
#[derive(Debug, Clone, Copy)]
pub(super) enum Route {
    /// The sample buildings of the profile: no job asked for.
    Sample,
    /// Dwellings only: dependants, officials and promoted titles.
    Houses,
    /// Dwellings, and this land job for everyone who takes one.
    Land(&'static str),
    /// Dwellings and copies of the workplace that staffs the job.
    Workplace(F),
}

/// The workplace whose master, workers or family staff the job, first in
/// key order.
pub(super) fn workplace_of(data: &'static Data, job: &str) -> Option<F> {
    data.occupations
        .workplaces
        .iter()
        .find(|(_, w)| {
            w.master.as_deref() == Some(job)
                || w.workers.iter().any(|x| x == job)
                || w.family_job.as_deref() == Some(job)
        })
        .and_then(|(k, _)| F::from_key(k))
}

pub(super) fn is_land(data: &'static Data, job: &str) -> bool {
    data.occupations.land_jobs.iter().any(|l| l.job == job)
}

pub(super) fn dependent(data: &'static Data, job: &str) -> bool {
    data.job(job).is_ok_and(|j| j.category == "dependent")
}

pub(super) fn official(data: &'static Data, job: &str, tier: Tier) -> bool {
    data.tiers.capital_officials.iter().any(|j| j == job)
        || data
            .tiers
            .tiers
            .get(tier.key())
            .is_some_and(|t| t.officials.iter().any(|j| j == job))
}

pub(super) fn promoted(data: &'static Data, job: &str, tier: Tier) -> bool {
    data.tiers
        .tiers
        .get(tier.key())
        .is_some_and(|t| t.promote.iter().any(|j| j == job))
}

/// Whether some synthetic settlement of `tier` has the job.
pub(super) fn reachable(data: &'static Data, job: &str, tier: Tier) -> bool {
    dependent(data, job)
        || is_land(data, job)
        || workplace_of(data, job).is_some()
        || official(data, job, tier)
        || promoted(data, job, tier)
}

/// Whether the job's holder is a notable where the job comes from: the
/// master of a notable workplace, an official or a promoted title.
pub(super) fn notable_source(data: &'static Data, job: &str, tier: Tier) -> bool {
    official(data, job, tier)
        || promoted(data, job, tier)
        || data
            .occupations
            .workplaces
            .values()
            .any(|w| w.notable && w.master.as_deref() == Some(job))
}

pub(super) fn route(data: &'static Data, job: &'static str, tier: Tier) -> Route {
    if dependent(data, job) || official(data, job, tier) {
        Route::Houses
    } else if is_land(data, job) {
        Route::Land(job)
    } else if let Some(f) = workplace_of(data, job) {
        Route::Workplace(f)
    } else {
        Route::Houses
    }
}

/// Where an office holder works when no job is asked for: a building
/// whose master the office fits, or none (a mature household head holds
/// it).
pub(super) fn office_building(kind: &str) -> Option<F> {
    Some(match kind {
        "ruler" | "steward" => F::Keep,
        "lord" => F::Manor,
        "captain" => F::Barracks,
        "high_priest" | "abbot" => F::Temple,
        "shrine_keeper" => F::Shrine,
        "harbourmaster" | "boatmaster" => F::Dock,
        "mine_overseer" | "miner_captain" => F::Mine,
        "court_mage" => F::Library,
        "guildmaster" => F::MarketHall,
        "craftmaster" => F::Workshop,
        "crime_boss" => F::Tavern,
        "fisher_head" => F::Boathouse,
        "forester" => F::LumberCamp,
        _ => return None,
    })
}

/// The office as a notable slot: titled from the job table when a job
/// shares its key, else from the key itself; at the first copy of the
/// route's workplace.
pub(super) fn office_slot(
    data: &'static Data,
    kind: &str,
    route: Route,
    buildings: &[BuildingSpec],
    sex: Option<Sex>,
) -> NotableSlot {
    let female = sex.map(|s| s == Sex::Female);
    let title = data.occupations.jobs.get(kind).map_or_else(
        || {
            kind.split('_')
                .map(|w| {
                    let mut c = w.chars();
                    c.next().map_or_else(String::new, |f| {
                        f.to_uppercase().chain(c).collect::<String>()
                    })
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
        |j| match (&j.title_female, female) {
            (Some(t), Some(true)) => t.clone(),
            _ => j.title.clone(),
        },
    );
    let building = match route {
        Route::Workplace(f) => buildings.iter().find(|b| b.function == f).map(|b| b.id),
        _ => None,
    };
    NotableSlot {
        id: format!("r{}.{kind}", SYNTHETIC_SETTLEMENT.0),
        kind: kind.to_owned(),
        title,
        building,
        given_name: None,
        family_name: None,
        female,
    }
}

/// The synthetic settlement's buildings, ids from 1.
pub(super) fn buildings(
    data: &'static Data,
    route: Route,
    profile: &SettlementProfile,
    seed: u64,
) -> Vec<BuildingSpec> {
    if matches!(route, Route::Sample) {
        return crate::sample::buildings(seed, profile);
    }
    let mut out: Vec<BuildingSpec> = Vec::new();
    let push = |out: &mut Vec<BuildingSpec>, function: F, capacity: u16, slots: u16| {
        out.push(BuildingSpec {
            id: BuildingId(u64::try_from(out.len()).unwrap_or(0) + 1),
            settlement_id: SYNTHETIC_SETTLEMENT,
            function,
            tags: Vec::new(),
            capacity,
            workplace_slots: slots,
            wealth: profile.wealth,
        });
    };
    if let Route::Workplace(f) = route {
        let workers = data
            .occupations
            .workplaces
            .get(f.key())
            .map_or(0, |w| w.workers.len());
        let slots = u16::try_from(workers + 1).unwrap_or(u16::MAX);
        for _ in 0..WORKPLACE_COPIES {
            push(&mut out, f, crate::sample::capacity(f), slots);
        }
    }
    let mut room: u32 = out.iter().map(|b| u32::from(b.capacity)).sum();
    while room < PEOPLE + u32::from(HOUSE_CAPACITY) {
        push(&mut out, F::House, HOUSE_CAPACITY, 0);
        room += u32::from(HOUSE_CAPACITY);
    }
    out
}
