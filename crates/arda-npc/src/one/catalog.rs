//! The occupation catalogue: every job and office key [`super::generate_one`]
//! takes, with the tiers and social ranks it accepts (logic/13
//! §npc-occupations; `GET /v1/npc/occupations`).

use serde::{Deserialize, Serialize};

use crate::data::Data;
use crate::error::NpcError;
use crate::input::Tier;
use crate::npc::{JobCategory, SocialRank};

use super::layout::reachable;
use super::TIERS;

/// Whether a key is a `job` or an `office` of [`super::OneRequest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OccupationKind {
    /// An occupation (`OneRequest::job`, `Npc.job.key`).
    Job,
    /// A society office (`OneRequest::office`, a notable slot's kind).
    Office,
}

/// A tier a key is accepted in, and the rank it carries there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OccupationTier {
    /// Settlement size class.
    pub tier: Tier,
    /// A job's social rank in the tier; for an office, the lowest rank of
    /// its holder there (a holder ranks at the higher of job and office).
    pub rank: SocialRank,
}

/// One key of the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occupation {
    /// The key `generate_one` takes. A key may be both a job and an office
    /// (`reeve`); `(kind, key)` is unique.
    pub key: String,
    /// Job or office.
    pub kind: OccupationKind,
    /// Display label: a job's title, an office's title without its place.
    pub label: String,
    /// A job's female title where it differs.
    pub label_female: Option<String>,
    /// A job's category; `None` for an office.
    pub category: Option<JobCategory>,
    /// Tiers the key is accepted in, smallest first.
    pub tiers: Vec<OccupationTier>,
}

/// Every job, then every office, each in key order.
///
/// # Errors
/// [`NpcError::Data`] for invalid bundled data.
pub fn occupations() -> Result<Vec<Occupation>, NpcError> {
    let data = Data::get()?;
    let occ = &data.occupations;
    let mut out = Vec::with_capacity(occ.jobs.len() + occ.offices.len());
    for (key, job) in &occ.jobs {
        let category = JobCategory::from_key(&job.category)
            .ok_or_else(|| NpcError::Data(format!("job {key}: category {}", job.category)))?;
        out.push(Occupation {
            key: key.clone(),
            kind: OccupationKind::Job,
            label: job.title.clone(),
            label_female: job.title_female.clone(),
            category: Some(category),
            tiers: TIERS
                .into_iter()
                .filter(|&t| reachable(data, key, t))
                .map(|tier| OccupationTier {
                    tier,
                    rank: SocialRank::from_level(job.rank_in(tier.key())),
                })
                .collect(),
        });
    }
    for (key, office) in &occ.offices {
        out.push(Occupation {
            key: key.clone(),
            kind: OccupationKind::Office,
            label: office.label.clone(),
            label_female: None,
            category: None,
            tiers: TIERS
                .into_iter()
                .map(|tier: Tier| OccupationTier {
                    tier,
                    rank: SocialRank::from_level(office.rank_in(tier.key())),
                })
                .collect(),
        });
    }
    Ok(out)
}
