//! Reserve a fresh output and publish its completion manifest only after success.
#![deny(missing_docs)]

use arda_core::{write_manifest, LoadError, Manifest, MANIFEST_NAME};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MARKER: &str = ".arda-generating";
const SCRATCH: &str = ".arda-hydrology-scratch";
const PENDING: &str = ".arda-manifest-pending";

/// Original I/O failures and the established clean-rerun refusal.
#[derive(Debug, thiserror::Error)]
pub enum PublicationError {
    /// An existing output contains at least one entry.
    #[error("output directory {0} is not empty; refusing to overwrite a world")]
    Occupied(PathBuf),
    /// A generated relative name does not identify an ordinary output layer.
    #[error("invalid generated layer path")]
    InvalidLayer,
    /// Original filesystem error at the actual attempted path.
    #[error("world output I/O on {path}: {source}")]
    Io {
        /// Affected output path.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        source: std::io::Error,
    },
    /// Existing canonical manifest serialization or writing failed.
    #[error(transparent)]
    Manifest(#[from] LoadError),
}

fn io(path: &Path, source: std::io::Error) -> PublicationError {
    PublicationError::Io {
        path: path.to_owned(),
        source,
    }
}

/// One reserved generation output. Failure deliberately retains its partial files,
/// so the existing clean-rerun rule applies. There is no automatic Drop cleanup.
/// Runtime scratch is inside this output; it is unrelated to Capstone feature docs.
pub struct WorldOutput {
    root: PathBuf,
    scratch: PathBuf,
}

impl WorldOutput {
    /// Private path used by pure resource admission before output creation.
    pub fn scratch_path(root: &Path) -> PathBuf {
        root.join(SCRATCH)
    }
    /// Accept an absent or empty output; reserve it before any expensive generation.
    /// Read-directory failures are errors, never evidence that a directory is empty.
    pub fn begin(root: &Path) -> Result<Self, PublicationError> {
        fs::create_dir_all(root).map_err(|e| io(root, e))?;
        let mut entries = fs::read_dir(root).map_err(|e| io(root, e))?;
        if let Some(entry) = entries.next() {
            entry.map_err(|e| io(root, e))?;
            return Err(PublicationError::Occupied(root.to_owned()));
        }
        let marker = root.join(MARKER);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .map_err(|e| io(&marker, e))?;
        let scratch = Self::scratch_path(root);
        fs::create_dir(&scratch).map_err(|e| io(&scratch, e))?;
        Ok(Self {
            root: root.to_owned(),
            scratch,
        })
    }

    /// Fresh private directory for admitted routing, flow and preparation files.
    pub fn scratch(&self) -> &Path {
        &self.scratch
    }

    /// Create one final layer. Repeated writes cannot silently overwrite an earlier layer.
    /// The manifest and private completion names are reserved for `commit`.
    pub fn write_layer(&self, relative: &Path, bytes: &[u8]) -> Result<(), PublicationError> {
        let mut file = self.create_layer(relative)?;
        let path = self.root.join(relative);
        file.write_all(bytes).map_err(|e| io(&path, e))?;
        file.flush().map_err(|e| io(&path, e))
    }

    /// Open one create-new layer for an admitted bounded streaming encoder.
    /// The caller must finish and drop this handle before committing the output.
    pub fn create_layer(&self, relative: &Path) -> Result<File, PublicationError> {
        let mut parts = relative.components();
        let Some(Component::Normal(first)) = parts.next() else {
            return Err(PublicationError::InvalidLayer);
        };
        if [MANIFEST_NAME, MARKER, SCRATCH, PENDING]
            .iter()
            .any(|s| first == *s)
            || parts.any(|p| !matches!(p, Component::Normal(_)))
        {
            return Err(PublicationError::InvalidLayer);
        }
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| io(&path, e))
    }

    /// Publish after every stage, output table and required flush has succeeded.
    /// All scratch owners must have been dropped before this call (also on Windows).
    /// The same-directory final rename is the last fallible operation. This provides
    /// process-interruption completion semantics, not power-loss durability guarantees.
    pub fn commit(self, manifest: &Manifest) -> Result<(), PublicationError> {
        write_manifest(&self.scratch, manifest)?;
        let staged = self.scratch.join(MANIFEST_NAME);
        let pending = self.root.join(PENDING);
        fs::rename(&staged, &pending).map_err(|e| io(&staged, e))?;
        fs::remove_dir_all(&self.scratch).map_err(|e| io(&self.scratch, e))?;
        let marker = self.root.join(MARKER);
        fs::remove_file(&marker).map_err(|e| io(&marker, e))?;
        let final_path = self.root.join(MANIFEST_NAME);
        fs::rename(&pending, &final_path).map_err(|e| io(&final_path, e))
    }
}

#[cfg(test)]
#[path = "publication_tests.rs"]
mod tests;
