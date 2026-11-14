//! Mirrors of the occupation catalogue (`arda_npc::occupations`,
//! logic/13 §npc-occupations), served by `GET /v1/npc/occupations`.

use super::{JobCategory, SocialRank, Tier};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Body of `GET /v1/npc/occupations`: every key `POST /v1/npc/generate`
/// takes as `job` or `office`. Every job, then every office, each in key
/// order; the same on every world (bundled data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OccupationList {
    /// The catalogue.
    pub occupations: Vec<Occupation>,
}

/// Whether a key is a `job` or an `office` of `GenerateNpcRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OccupationKind {
    /// An occupation: `GenerateNpcRequest.job`, `Npc.job.key`.
    Job,
    /// A society office: `GenerateNpcRequest.office`.
    Office,
}

/// A tier a key is accepted in, and the rank it carries there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OccupationTier {
    /// Settlement size class (`GenerateNpcRequest.tier`).
    pub tier: Tier,
    /// A job's social rank in the tier; for an office, the lowest rank of
    /// its holder there (a holder ranks at the higher of job and office).
    pub rank: SocialRank,
}

/// One key of the catalogue. `(kind, key)` is unique; `reeve`, `elder`,
/// `harbourmaster` and others are both a job and an office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Occupation {
    /// The key `POST /v1/npc/generate` takes.
    pub key: String,
    /// Job or office.
    pub kind: OccupationKind,
    /// Display label: a job's title, an office's title without its place.
    pub label: String,
    /// A job's female title where it differs; otherwise `null`.
    pub label_female: Option<String>,
    /// A job's category; `null` for an office.
    pub category: Option<JobCategory>,
    /// Tiers the key is accepted in, smallest first, with its rank in
    /// each. A job outside them is refused (`bad_request`).
    pub tiers: Vec<OccupationTier>,
}
