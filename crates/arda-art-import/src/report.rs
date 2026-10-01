//! The import report: fixes applied, flags for human review, rejections,
//! validator results and vocabulary coverage. Written as `report.md` and
//! `report.json` next to the catalogue.

use serde::Serialize;
use std::fmt::Write as _;

/// Whether an asset made it into the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// In the catalogue.
    Imported,
    /// Left out (its slot falls back to the base library); the processed
    /// image is kept under `rejected/` for inspection.
    Rejected,
}

/// One processed raw file.
#[derive(Debug, Clone, Serialize)]
pub struct AssetReport {
    /// Catalogue id.
    pub id: String,
    /// Raw file name.
    pub file: String,
    /// Asset class.
    pub class: String,
    /// Imported or rejected.
    pub status: Status,
    /// What the importer did.
    pub fixes: Vec<String>,
    /// What a human should look at.
    pub flags: Vec<String>,
}

/// A raw file that could not be mapped to an asset.
#[derive(Debug, Clone, Serialize)]
pub struct Skipped {
    /// Raw file name.
    pub file: String,
    /// Why.
    pub reason: String,
}

/// Which vocabulary slots the import covers.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Coverage {
    /// Slots the imported library provides.
    pub imported: Vec<String>,
    /// Slots that fall back to the base library.
    pub fallback: Vec<String>,
    /// Slots no library provides.
    pub missing: Vec<String>,
}

/// The whole report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    /// Library name and version.
    pub library: String,
    /// Every processed file, in catalogue order.
    pub assets: Vec<AssetReport>,
    /// Files that could not be mapped.
    pub skipped: Vec<Skipped>,
    /// The colour grade, if any.
    pub grade: Option<String>,
    /// Validator issues of the imported library on its own (excluding
    /// kits that are completed by the base).
    pub validation: Vec<String>,
    /// Kits the import only partly provides; the base fills the rest.
    pub partial_kits: Vec<String>,
    /// Validator issues of the imported library stacked over the base.
    pub stack_validation: Option<Vec<String>>,
    /// Vocabulary coverage.
    pub coverage: Coverage,
}

impl Report {
    /// Assets with at least one flag.
    #[must_use]
    pub fn flagged(&self) -> usize {
        self.assets.iter().filter(|a| !a.flags.is_empty()).count()
    }

    /// Whether the written library (and the stack, if checked) validates.
    #[must_use]
    pub fn valid(&self) -> bool {
        self.validation.is_empty() && self.stack_validation.as_ref().is_none_or(Vec::is_empty)
    }

    /// The report as Markdown.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut s = String::new();
        let imported = self
            .assets
            .iter()
            .filter(|a| a.status == Status::Imported)
            .count();
        let _ = writeln!(s, "# Import report: {}\n", self.library);
        let _ = writeln!(
            s,
            "{imported} imported, {} rejected, {} skipped, {} flagged for review. Validator: {}.\n",
            self.assets.len() - imported,
            self.skipped.len(),
            self.flagged(),
            if self.valid() { "pass" } else { "FAIL" }
        );
        if let Some(g) = &self.grade {
            let _ = writeln!(s, "Colour grade: {g}\n");
        }
        let review: Vec<&AssetReport> =
            self.assets.iter().filter(|a| !a.flags.is_empty()).collect();
        if !review.is_empty() {
            let _ = writeln!(s, "## Needs review\n");
            for a in review {
                let _ = writeln!(s, "- **{}** (`{}`, {:?})", a.id, a.file, a.status);
                for f in &a.flags {
                    let _ = writeln!(s, "  - {f}");
                }
            }
            s.push('\n');
        }
        if !self.skipped.is_empty() {
            let _ = writeln!(s, "## Skipped files\n");
            for k in &self.skipped {
                let _ = writeln!(s, "- `{}`: {}", k.file, k.reason);
            }
            s.push('\n');
        }
        let _ = writeln!(s, "## Validation\n");
        list(&mut s, "Imported library on its own", &self.validation);
        if !self.partial_kits.is_empty() {
            let _ = writeln!(s, "Partial kits (completed by the base library):\n");
            for k in &self.partial_kits {
                let _ = writeln!(s, "- {k}");
            }
            s.push('\n');
        }
        if let Some(v) = &self.stack_validation {
            list(&mut s, "Stacked over the base library", v);
        }
        let c = &self.coverage;
        let _ = writeln!(s, "## Vocabulary coverage\n");
        let _ = writeln!(
            s,
            "{} slots imported, {} fall back to the base library, {} missing everywhere.\n",
            c.imported.len(),
            c.fallback.len(),
            c.missing.len()
        );
        for (title, v) in [
            ("Falls back to placeholders", &c.fallback),
            ("Missing everywhere", &c.missing),
        ] {
            if !v.is_empty() {
                let _ = writeln!(s, "**{title}:** {}\n", v.join(", "));
            }
        }
        let _ = writeln!(s, "## Fixes applied\n");
        for a in &self.assets {
            let _ = writeln!(s, "- **{}** (`{}`)", a.id, a.file);
            for f in &a.fixes {
                let _ = writeln!(s, "  - {f}");
            }
        }
        s
    }
}

fn list(s: &mut String, title: &str, issues: &[String]) {
    if issues.is_empty() {
        let _ = writeln!(s, "{title}: pass.\n");
    } else {
        let _ = writeln!(s, "{title}: {} issue(s)\n", issues.len());
        for i in issues {
            let _ = writeln!(s, "- {i}");
        }
        s.push('\n');
    }
}
