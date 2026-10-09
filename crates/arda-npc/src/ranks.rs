//! Social rank rules (logic/13 §npc-ranks), read from the bundled tables so
//! callers and tests check NPCs against the same data that made them.
//!
//! A person's rank is their job's rank in the settlement's tier, raised to
//! the rank of the society office they hold, if any. A working person's
//! wealth stays inside the lifestyle band of that rank.

use crate::data::Data;
use crate::error::NpcError;
use crate::input::Tier;
use crate::npc::{Lifestyle, SocialRank};

/// Rank of a job (`occupations.json` key) in a tier.
///
/// # Errors
/// [`NpcError::Data`] for an unknown job or invalid bundled data.
pub fn job_rank(job: &str, tier: Tier) -> Result<SocialRank, NpcError> {
    Ok(SocialRank::from_level(
        Data::get()?.job(job)?.rank_in(tier.key()),
    ))
}

/// Rank of a society office (a notable slot's `kind`) in a tier; `None`
/// for a kind the table does not know, which leaves the job's rank alone.
///
/// # Errors
/// [`NpcError::Data`] for invalid bundled data.
pub fn office_rank(kind: &str, tier: Tier) -> Result<Option<SocialRank>, NpcError> {
    Ok(Data::get()?
        .occupations
        .offices
        .get(kind)
        .map(|o| SocialRank::from_level(o.rank_in(tier.key()))))
}

/// Whether an office kind is civic (counts toward a tier's civic cap).
///
/// # Errors
/// [`NpcError::Data`] for invalid bundled data.
pub fn civic_office(kind: &str) -> Result<bool, NpcError> {
    Ok(Data::get()?
        .occupations
        .offices
        .get(kind)
        .is_some_and(|o| o.civic))
}

/// The most civic figures a tier appoints or promotes.
///
/// # Errors
/// [`NpcError::Data`] for invalid bundled data.
pub fn civic_cap(tier: Tier) -> Result<u32, NpcError> {
    Data::get()?
        .tiers
        .tiers
        .get(tier.key())
        .map(|t| t.civic_cap)
        .ok_or_else(|| NpcError::Data(format!("no tier {}", tier.key())))
}

/// The `[lowest, highest]` lifestyle of a working person of `rank`;
/// `None` for dependants, who live as their household does.
///
/// # Errors
/// [`NpcError::Data`] for invalid bundled data.
pub fn lifestyle_band(rank: SocialRank) -> Result<Option<[Lifestyle; 2]>, NpcError> {
    Ok(Data::get()?.occupations.rank_lifestyles.get(&rank).copied())
}
