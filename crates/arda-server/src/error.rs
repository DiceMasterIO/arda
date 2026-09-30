//! Typed service errors and their JSON wire form.
//!
//! Status mapping: malformed or out-of-world coordinates are 400, absent
//! routes, tiles and layer files are 404, oversized tactical requests are
//! 413, inconsistent tactical layouts are 422, world-derived tactical blocks
//! are 501 until their source exists, and everything else is 500.

use crate::tactical::block::BlockError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use ts_rs::TS;

/// Every failure the service reports.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The request is malformed: unparsable path or query values.
    #[error("{0}")]
    BadRequest(String),
    /// A coordinate lies outside the world.
    #[error("{0}")]
    OutOfRange(String),
    /// The requested resource does not exist.
    #[error("{0}")]
    NotFound(String),
    /// The stored world could not supply a layer.
    #[error(transparent)]
    Load(#[from] arda_core::LoadError),
    /// A render or export failed.
    #[error(transparent)]
    Export(#[from] arda::ExportError),
    /// The fine terrain file failed after validation.
    #[error("fine terrain: {0}")]
    Terrain(#[from] arda_core::TerrainFileError),
    /// A request body, its contents or a requested render exceed a documented limit.
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    /// The NPC generator refused its inputs.
    #[error(transparent)]
    Npc(#[from] arda_npc::NpcError),
    /// A configured memory or size limit refused the work.
    #[error("resource limit: {0}")]
    ResourceLimit(String),
    /// Image decoding or encoding failed.
    #[error("image: {0}")]
    Image(String),
    /// The request body is not in the media type the route accepts.
    #[error("unsupported media type: {0}")]
    UnsupportedMediaType(String),
    /// A tactical layout is inconsistent with itself or the loaded library.
    #[error("invalid layout: {0}")]
    InvalidLayout(String),
    /// The tactical library failed to load or a render failed.
    #[error("tactical: {0}")]
    Tactical(#[from] arda_tactical::TacticalError),
    /// A block source could not supply a world-derived tactical block.
    #[error(transparent)]
    Block(#[from] crate::tactical::block::BlockError),
    /// An invariant broke inside the service.
    #[error("internal: {0}")]
    Internal(String),
}

/// JSON body of every error response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ApiError {
    /// The error details.
    pub error: ApiErrorBody,
}

/// The 501 body of a feature whose source crate does not exist yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct NotYetError {
    /// The error details; `code` is `not_implemented`.
    pub error: ApiErrorBody,
    /// Crate planned to provide the feature, for example `arda-refine`.
    pub planned_source: String,
}

/// Machine-readable code, HTTP status and human message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ApiErrorBody {
    /// Stable code: `bad_request`, `out_of_range`, `not_found`, `no_block`, `payload_too_large`,
    /// `unsupported_media_type`, `invalid_layout`, `resource_limit`, `not_implemented` or `internal`.
    pub code: String,
    /// HTTP status repeated in the body.
    pub status: u16,
    /// Human-readable detail; not stable.
    pub message: String,
}

fn missing_file(error: &arda_core::LoadError) -> bool {
    use arda_core::{FormatError, LoadError};
    match error {
        LoadError::ManifestMissing { .. } => true,
        LoadError::Corrupt {
            source: FormatError::Io { source, .. },
        } => source.kind() == std::io::ErrorKind::NotFound,
        _ => false,
    }
}

impl ServerError {
    /// HTTP status and stable code.
    #[must_use]
    pub fn classify(&self) -> (StatusCode, &'static str) {
        match self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::OutOfRange(_) | Self::Load(arda_core::LoadError::OutOfRange { .. }) => {
                (StatusCode::BAD_REQUEST, "out_of_range")
            }
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Load(e) if missing_file(e) => (StatusCode::NOT_FOUND, "not_found"),
            Self::PayloadTooLarge(_) => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large"),
            Self::Npc(arda_npc::NpcError::UnknownNpc(_)) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Npc(arda_npc::NpcError::Data(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal")
            }
            Self::Npc(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::ResourceLimit(_) => (StatusCode::INTERNAL_SERVER_ERROR, "resource_limit"),
            Self::UnsupportedMediaType(_) => {
                (StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_media_type")
            }
            Self::InvalidLayout(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_layout"),
            Self::Block(BlockError::NotYet { .. }) => {
                (StatusCode::NOT_IMPLEMENTED, "not_implemented")
            }
            // logic/16 §api-errors: a cell with no land square.
            Self::Block(BlockError::NoBlock { .. }) => (StatusCode::NOT_FOUND, "no_block"),
            Self::Block(BlockError::Window(_)) => (StatusCode::BAD_REQUEST, "bad_request"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }

    /// The wire body for this error.
    #[must_use]
    pub fn body(&self) -> ApiError {
        let (status, code) = self.classify();
        ApiError {
            error: ApiErrorBody {
                code: code.to_owned(),
                status: status.as_u16(),
                message: self.to_string(),
            },
        }
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, _) = self.classify();
        if let Self::Block(BlockError::NotYet { planned_source }) = &self {
            let body = NotYetError {
                error: self.body().error,
                planned_source: planned_source.clone(),
            };
            return (status, Json(body)).into_response();
        }
        (status, Json(self.body())).into_response()
    }
}

/// Shorthand for service results.
pub type ServerResult<T> = Result<T, ServerError>;

/// Locks a mutex, reporting poisoning as an internal error instead of panicking.
///
/// # Errors
/// [`ServerError::Internal`] when a previous holder panicked.
pub fn lock<T>(m: &std::sync::Mutex<T>) -> ServerResult<std::sync::MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| ServerError::Internal("cache lock poisoned".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_follow_the_documented_mapping() {
        let range = ServerError::Load(arda_core::LoadError::OutOfRange {
            what: "area",
            x: 9,
            y: 0,
            max_x: 1,
            max_y: 3,
        });
        assert_eq!(range.classify(), (StatusCode::BAD_REQUEST, "out_of_range"));
        let missing = ServerError::Load(arda_core::LoadError::Corrupt {
            source: arda_core::FormatError::Io {
                path: "x".into(),
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            },
        });
        assert_eq!(missing.classify().0, StatusCode::NOT_FOUND);
        let body = ServerError::Internal("boom".into()).body();
        assert_eq!(
            (body.error.status, body.error.code.as_str()),
            (500, "internal")
        );
        let unknown = ServerError::Npc(arda_npc::NpcError::UnknownNpc(arda_npc::NpcId(1)));
        assert_eq!(unknown.classify(), (StatusCode::NOT_FOUND, "not_found"));
        let housing = ServerError::Npc(arda_npc::NpcError::NotEnoughHousing {
            population: 2,
            capacity: 1,
        });
        assert_eq!(housing.classify(), (StatusCode::BAD_REQUEST, "bad_request"));
        let big = ServerError::PayloadTooLarge("x".into());
        assert_eq!(big.classify().0, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn tactical_errors_have_their_own_statuses() {
        let not_yet = ServerError::Block(BlockError::NotYet {
            planned_source: "arda-refine".into(),
        });
        assert_eq!(
            not_yet.classify(),
            (StatusCode::NOT_IMPLEMENTED, "not_implemented")
        );
        assert_eq!(
            ServerError::PayloadTooLarge("x".into()).classify().0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            ServerError::InvalidLayout("x".into()).classify().0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
