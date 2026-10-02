//! `arda tactical import`: turns a folder of raw AI-generated images
//! (ComfyUI with SDXL or FLUX, say) into a valid arda tactical asset library
//! (goals 58–61), with a report of every fix and every doubt.
//!
//! The steps, per class (README of `arda-tactical`, "Importing AI art"):
//! - **cut-outs** (props, vegetation, walls): background removal by a
//!   border flood, baked-shadow stripping, enclosed backdrop pockets
//!   (class-aware), soft matte and defringe, halo
//!   and haze removal, auto-crop, centring and a Mitchell fit to
//!   `footprint × ppsq`; wall pieces are turned to their canonical arms;
//! - **textures**: crop and resize to the tile, seam fix (offset-and-blend,
//!   or an edge blend that keeps structured layouts), broad-contrast
//!   flattening, and registration of structured variants;
//! - an optional global **colour grade** toward a reference or palette;
//! - **catalogue** metadata inherited from the base library, provenance
//!   from the manifest and PNG metadata, then the **validator**.
//!
//! Output is deterministic: files are processed in sorted order and every
//! per-asset step is single-threaded maths (assets run in parallel, which
//! cannot change a result).

#![deny(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod cleanup;
pub mod contact;
pub mod error;
pub mod fit;
pub mod font;
pub mod grade;
pub mod holes;
pub mod image_io;
pub mod manifest;
pub mod matte;
pub mod meta;
pub mod naming;
pub mod ops;
pub mod output;
pub mod pipeline;
pub mod report;
pub mod shadow;
pub mod structured;
pub mod texture;
pub mod vocab;
pub mod wall;

pub use error::{ImportError, ImportResult};
pub use report::Report;

use manifest::Manifest;
use meta::{Base, Origin};
use naming::Target;
use pipeline::Processed;
use rayon::prelude::*;
use report::Skipped;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Pixels per square when neither the manifest nor a base library says.
pub const DEFAULT_PPSQ: u32 = 128;

/// What to import and where.
#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Directory of raw images (searched recursively, in sorted order).
    pub raw_dir: PathBuf,
    /// Library directory to write.
    pub out_dir: PathBuf,
    /// Optional `import.toml`.
    pub manifest: Option<PathBuf>,
    /// Base library (or `top:…:bottom` stack) for default metadata,
    /// coverage and the stacked validation; usually the placeholders.
    pub base: Option<PathBuf>,
    /// Reference image for the grade (overrides the manifest).
    pub reference: Option<PathBuf>,
    /// Grade strength (overrides the manifest).
    pub grade_strength: Option<f32>,
    /// Where to write a contact sheet, if wanted.
    pub contact_sheet: Option<PathBuf>,
}

/// A raw file mapped to an asset, before processing.
#[derive(Debug, Clone)]
struct Planned {
    path: PathBuf,
    file: String,
    target: Target,
    id: String,
    vocab_flag: Option<String>,
}

/// Runs the whole import and writes the library, report and contact sheet.
///
/// # Errors
/// Unreadable inputs or unwritable outputs; problems with single images are
/// reported, not raised.
pub fn import(opts: &ImportOptions) -> ImportResult<Report> {
    let manifest = match &opts.manifest {
        Some(p) => Manifest::read(p)?,
        None => Manifest::default(),
    };
    let base_catalogs = match &opts.base {
        Some(b) => arda_tactical::library_stack::read_stack_catalogs(b)?,
        None => Vec::new(),
    };
    let base = Base::new(&base_catalogs);
    let ppsq = manifest
        .library
        .pixels_per_square
        .or_else(|| base_catalogs.first().map(|c| c.pixels_per_square))
        .unwrap_or(DEFAULT_PPSQ);
    if ppsq == 0 {
        return Err(ImportError::Options(
            "pixels_per_square must be positive".into(),
        ));
    }
    let mut skipped = Vec::new();
    let planned = plan(&opts.raw_dir, &manifest, &mut skipped)?;
    let processed: Vec<ImportResult<Processed>> = planned
        .par_iter()
        .map(|p| process_one(p, &manifest, &base, ppsq))
        .collect();
    let mut done = Vec::new();
    for (p, r) in planned.iter().zip(processed) {
        match r {
            Ok(x) => done.push(x),
            Err(e) => skipped.push(Skipped {
                file: p.file.clone(),
                reason: e.to_string(),
            }),
        }
    }
    output::register_structured(&mut done);
    let grade = output::grade_all(&mut done, &manifest, opts)?;
    let ctx = output::Context {
        opts,
        manifest: &manifest,
        base_catalogs: &base_catalogs,
        base: &base,
        ppsq,
    };
    output::write(&ctx, done, skipped, grade)
}

/// Lists raw images recursively in sorted order.
fn list_images(dir: &Path, out: &mut Vec<PathBuf>) -> ImportResult<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(error::io(dir))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            list_images(&p, out)?;
        } else if p
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
        {
            out.push(p);
        }
    }
    Ok(())
}

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Maps files to targets and assigns ids: texture variants `<base>.<n>`,
/// further takes of one cut-out `<base>.alt<n>`.
fn plan(
    raw_dir: &Path,
    manifest: &Manifest,
    skipped: &mut Vec<Skipped>,
) -> ImportResult<Vec<Planned>> {
    let mut files = Vec::new();
    list_images(raw_dir, &mut files)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for path in files {
        let file = path
            .strip_prefix(raw_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let name = file_name(&path);
        let stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let spec = manifest
            .for_file(&name)
            .or_else(|| manifest.for_file(&file))
            .and_then(|e| e.id.clone())
            .unwrap_or(stem);
        let target = match naming::parse(&spec) {
            Ok(t) => t,
            Err(reason) => {
                skipped.push(Skipped { file, reason });
                continue;
            }
        };
        let base_id = target.base_id();
        let n = counts.entry(base_id.clone()).or_insert(0);
        let id = match (&target, *n) {
            (Target::Texture { .. }, k) => format!("{base_id}.{k}"),
            (_, 0) => base_id.clone(),
            (_, k) => format!("{base_id}.alt{k}"),
        };
        *n += 1;
        let vocab_flag = vocabulary_flag(&target);
        out.push(Planned {
            path,
            file,
            target,
            id,
            vocab_flag,
        });
    }
    Ok(out)
}

/// "not in the vocabulary", with a suggestion.
fn vocabulary_flag(t: &Target) -> Option<String> {
    let (word, known, what) = match t {
        Target::Texture { key } => (key.as_str(), vocab::GROUND_KEYS, "ground key"),
        Target::Wall { kit, .. } => (kit.as_str(), vocab::WALL_KITS, "wall kit"),
        Target::Prop { name } => (name.as_str(), vocab::PROPS, "prop"),
        Target::Vegetation { name } => (name.as_str(), vocab::VEGETATION, "vegetation"),
    };
    if known.contains(&word) {
        return None;
    }
    let hint = vocab::suggest(word, known)
        .map(|s| format!("; did you mean `{s}`?"))
        .unwrap_or_default();
    Some(format!(
        "{what} `{word}` is not in docs/goal-prompts/vocabulary.md, so no layout asks for it{hint}"
    ))
}

fn process_one(
    p: &Planned,
    manifest: &Manifest,
    base: &Base,
    ppsq: u32,
) -> ImportResult<Processed> {
    let raw = image_io::read(&p.path)?;
    let name = file_name(&p.path);
    let entry = manifest
        .for_file(&name)
        .or_else(|| manifest.for_file(&p.file))
        .or_else(|| manifest.for_id(&p.target.base_id()));
    let mut asset = match base.find(&p.target) {
        Some(b) => b.clone(),
        None => meta::class_default(&p.target),
    };
    asset.id.clone_from(&p.id);
    if let Some(e) = entry {
        meta::apply_entry(&mut asset, e);
    }
    let (prov, unknown_tool) = meta::provenance(Origin {
        file: &p.file,
        library: &manifest.library,
        entry,
        found: &raw.meta,
    });
    asset.provenance = prov;
    let licence = entry
        .and_then(|e| e.licence.clone())
        .or_else(|| manifest.library.licence.clone());
    asset.licence = licence.clone().unwrap_or_else(|| "unspecified".into());
    asset.image = format!("{}/{}.png", output::class_dir(&p.target), p.id);
    let mode = entry.and_then(|e| e.shadow).unwrap_or_default();
    let holes = entry.and_then(|e| e.holes);
    let mut out = pipeline::process(
        &raw,
        asset,
        p.target.clone(),
        p.file.clone(),
        (ppsq, mode, holes),
    );
    if let Some(f) = &p.vocab_flag {
        out.flags.push(f.clone());
    }
    if licence.is_none() {
        out.flags
            .push("no licence given: set `licence` in import.toml".into());
    }
    if unknown_tool {
        out.flags
            .push("generator unknown: set `tool` and `model` in import.toml".into());
    }
    Ok(out)
}
