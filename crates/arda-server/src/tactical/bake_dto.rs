//! Wire types and job identity of `POST /v1/tactical/bake` (1.0 task 04;
//! see [`super::bake`]).

use super::bake::BAKE_FORMAT;
use super::dto::{TacticalLayoutDto, TacticalTilesDto};
use crate::error::{ApiErrorBody, ServerError, ServerResult};
use arda_tactical::TacticalLayout;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Body of `POST /v1/tactical/bake` (TS `BakeRequest`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BakeRequest {
    /// Body version; `1` when present.
    #[serde(default)]
    #[ts(optional)]
    pub format_version: Option<u32>,
    /// Cell column.
    pub gx: u32,
    /// Cell row.
    pub gy: u32,
    /// Output pixels per square: 64, 96 or 128 (default 128).
    #[serde(default)]
    #[ts(optional)]
    pub ppsq: Option<u32>,
    /// Pull ground and water toward the world map's colours, as
    /// `?world_grade=1` on the cell route (default false).
    #[serde(default)]
    #[ts(optional)]
    pub world_grade: Option<bool>,
    /// The edited cell: exactly 64 × 64 squares with `origin` equal to
    /// `[gx * 64, gy * 64]`, as `GET /v1/tactical/cell/{gx}/{gy}` sends it.
    pub layout: TacticalLayoutDto,
}

/// Where a bake job stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BakeState {
    /// Waiting for the bake lane.
    Queued,
    /// Rendering and encoding.
    Running,
    /// Tiles ready to download.
    Done,
    /// Finished with `error`.
    Failed,
}

/// `202` (or `200` once done) body of `POST /v1/tactical/bake`
/// (TS `BakeAccepted`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct BakeAccepted {
    /// Body version ([`BAKE_FORMAT`]).
    pub format_version: u32,
    /// Content-addressed job id (32 hex digits): the same body against the
    /// same world and library always gets the same id.
    pub job_id: String,
    /// Where the job stands now.
    pub state: BakeState,
    /// Where to poll: `/v1/tactical/bake/{job_id}`.
    pub status_url: String,
}

/// `GET /v1/tactical/bake/{id}` (TS `BakeStatus`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, JsonSchema)]
pub struct BakeStatus {
    /// Body version ([`BAKE_FORMAT`]).
    pub format_version: u32,
    /// The job id.
    pub job_id: String,
    /// Where the job stands.
    pub state: BakeState,
    /// Cell column.
    pub gx: u32,
    /// Cell row.
    pub gy: u32,
    /// Top-left square of the cell in world squares (64 per cell).
    #[ts(type = "[number, number]")]
    pub origin_gs: [i64; 2],
    /// Whether the world grade applies.
    pub world_grade: bool,
    /// Pyramid geometry once `done`, else `null`.
    pub tiles: Option<TacticalTilesDto>,
    /// Tiles in the set once `done`, else 0.
    pub tile_count: u32,
    /// Encoded bytes of the whole set once `done`, else 0.
    pub total_bytes: u32,
    /// Why the job failed, when `failed`, else `null`.
    pub error: Option<ApiErrorBody>,
}

/// A checked bake request.
#[derive(Debug, Clone, PartialEq)]
pub struct BakeRequestParsed {
    /// Cell column.
    pub gx: u32,
    /// Cell row.
    pub gy: u32,
    /// Output pixels per square.
    pub ppsq: u32,
    /// Whether the world grade applies.
    pub world_grade: bool,
    /// The posted cell layout.
    pub layout: TacticalLayout,
}

/// The canonical request the job id hashes: every option resolved.
#[derive(Serialize)]
struct Canonical<'a> {
    format_version: u32,
    gx: u32,
    gy: u32,
    ppsq: u32,
    world_grade: bool,
    layout: &'a TacticalLayout,
}

/// The job id of `req`: the first 32 hex digits of the BLAKE3 hash of the
/// world seed, the library version and the canonical request JSON.
///
/// # Errors
/// [`ServerError::Internal`] if the request cannot be serialised.
pub fn job_id(
    world_seed: u64,
    library_version: &str,
    req: &BakeRequestParsed,
) -> ServerResult<String> {
    let canonical = Canonical {
        format_version: BAKE_FORMAT,
        gx: req.gx,
        gy: req.gy,
        ppsq: req.ppsq,
        world_grade: req.world_grade,
        layout: &req.layout,
    };
    let json = serde_json::to_vec(&canonical)
        .map_err(|e| ServerError::Internal(format!("bake json: {e}")))?;
    let mut h = blake3::Hasher::new();
    h.update(b"arda-server/tactical-bake/v1");
    h.update(&world_seed.to_le_bytes());
    h.update(library_version.as_bytes());
    h.update(&[0]);
    h.update(&json);
    Ok(h.finalize().to_hex().as_str()[..32].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(ground: &str) -> BakeRequestParsed {
        let mut layout = TacticalLayout::new("cell_1_2", 64, 64, ground);
        layout.origin = Some([64, 128]);
        BakeRequestParsed {
            gx: 1,
            gy: 2,
            ppsq: 64,
            world_grade: false,
            layout,
        }
    }

    #[test]
    fn job_ids_are_content_addressed() {
        let a = job_id(42, "v1", &req("grass")).unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(a, job_id(42, "v1", &req("grass")).unwrap());
        assert_ne!(a, job_id(43, "v1", &req("grass")).unwrap());
        assert_ne!(a, job_id(42, "v2", &req("grass")).unwrap());
        assert_ne!(a, job_id(42, "v1", &req("dirt")).unwrap());
        let mut graded = req("grass");
        graded.world_grade = true;
        assert_ne!(a, job_id(42, "v1", &graded).unwrap());
    }
}
