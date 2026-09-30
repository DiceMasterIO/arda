//! JSON output of a [`Society`].

use crate::error::SocietyError;
use crate::Society;
use std::path::Path;

/// Pretty JSON of `society`. Key order is fixed by the struct layout and
/// `BTreeMap`s, so the text is byte-identical for identical input.
///
/// # Errors
/// [`SocietyError::Serialise`] if serialisation fails.
pub fn to_json(society: &Society) -> Result<String, SocietyError> {
    Ok(serde_json::to_string_pretty(society)?)
}

/// Writes `society` as JSON to `path`, creating parent directories.
///
/// # Errors
/// [`SocietyError::Io`] or [`SocietyError::Serialise`].
pub fn write_json(society: &Society, path: &Path) -> Result<(), SocietyError> {
    let text = to_json(society)?;
    let io = |source| SocietyError::Io {
        path: path.display().to_string(),
        source,
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(io)?;
    }
    std::fs::write(path, text).map_err(io)
}
