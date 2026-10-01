//! `arda tactical import`: raw AI-generated images to a tactical library
//! (goals 58–61, `crates/arda-art-import`).

use anyhow::{bail, Context, Result};
use arda_art_import::report::Status;
use arda_art_import::{import, ImportOptions};
use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub(crate) struct ImportArgs {
    /// Directory of raw PNG or JPEG images, named `prop.anvil__x.png`,
    /// `ground.grass.x.png`, `wall.stone.corner.png`… (or mapped by the
    /// manifest). Searched recursively.
    raw_dir: PathBuf,
    /// Library directory to write (catalog.json, images, report.md).
    #[arg(long)]
    out: PathBuf,
    /// Optional import.toml: library name, licence, tool, model, grade and
    /// per-file overrides (see the arda-tactical README).
    #[arg(long)]
    manifest: Option<PathBuf>,
    /// Base library or stack for default metadata, coverage and the stacked
    /// validation.
    #[arg(long, default_value = "assets/tactical/placeholder")]
    base: PathBuf,
    /// Import without a base library (class defaults only).
    #[arg(long)]
    no_base: bool,
    /// Grade colours toward this reference image (PNG or JPEG).
    #[arg(long)]
    reference: Option<PathBuf>,
    /// Grade strength, 0–1 (default: the manifest's, else 0.5).
    #[arg(long)]
    grade_strength: Option<f32>,
    /// Write a contact sheet for curation to this PNG.
    #[arg(long)]
    contact_sheet: Option<PathBuf>,
}

pub(crate) fn run(a: &ImportArgs) -> Result<()> {
    let opts = ImportOptions {
        raw_dir: a.raw_dir.clone(),
        out_dir: a.out.clone(),
        manifest: a.manifest.clone(),
        base: (!a.no_base).then(|| a.base.clone()),
        reference: a.reference.clone(),
        grade_strength: a.grade_strength,
        contact_sheet: a.contact_sheet.clone(),
    };
    let report = import(&opts).with_context(|| format!("importing {}", a.raw_dir.display()))?;
    let imported = report
        .assets
        .iter()
        .filter(|x| x.status == Status::Imported)
        .count();
    println!(
        "{}: {imported} imported, {} rejected, {} skipped, {} flagged for review",
        a.out.display(),
        report.assets.len() - imported,
        report.skipped.len(),
        report.flagged()
    );
    let c = &report.coverage;
    println!(
        "coverage: {} vocabulary slots imported, {} fall back to the base, {} missing",
        c.imported.len(),
        c.fallback.len(),
        c.missing.len()
    );
    if let Some(sheet) = &a.contact_sheet {
        println!("contact sheet: {}", sheet.display());
    }
    println!("report: {}", a.out.join("report.md").display());
    if !report.valid() {
        for i in report
            .validation
            .iter()
            .chain(report.stack_validation.iter().flatten())
        {
            eprintln!("{i}");
        }
        bail!("the imported library failed validation");
    }
    Ok(())
}
