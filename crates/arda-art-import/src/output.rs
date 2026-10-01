//! The library-wide steps after per-file processing: structured-variant
//! registration, the colour grade, per-asset validation, writing the
//! library, the stacked validation, coverage, the report and the sheet.

use crate::contact;
use crate::error::{ImportError, ImportResult};
use crate::grade::{self, Accumulator, Grade};
use crate::image_io;
use crate::manifest::{parse_hex, Manifest};
use crate::meta::{is_structured, Base};
use crate::naming::Target;
use crate::pipeline::Processed;
use crate::report::{AssetReport, Coverage, Report, Skipped, Status};
use crate::structured::{register, MIN_LAYOUT_NCC};
use crate::vocab;
use crate::ImportOptions;
use arda_tactical::catalog::{self, Catalog, TagVocabulary, FORMAT_VERSION};
use arda_tactical::validate::{validate, Images, Rule, Thresholds};
use arda_tactical::{library, Library, TacticalError};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Default grade strength.
pub const DEFAULT_STRENGTH: f32 = 0.5;

/// Inputs shared by the writing steps.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The import options.
    pub opts: &'a ImportOptions,
    /// The manifest.
    pub manifest: &'a Manifest,
    /// Base catalogues, top first.
    pub base_catalogs: &'a [Catalog],
    /// Base records.
    pub base: &'a Base,
    /// Output pixels per square.
    pub ppsq: u32,
}

/// The library sub-directory for a target's images.
#[must_use]
pub fn class_dir(t: &Target) -> &'static str {
    match t {
        Target::Texture { .. } => "ground",
        Target::Wall { .. } => "walls",
        Target::Prop { .. } => "props",
        Target::Vegetation { .. } => "vegetation",
    }
}

/// Rolls every structured variant into register with its key's first
/// variant, and flags layouts that still differ.
pub fn register_structured(done: &mut [Processed]) {
    let mut first: BTreeMap<String, usize> = BTreeMap::new();
    for i in 0..done.len() {
        let key = match (&done[i].target, is_structured(&done[i].asset)) {
            (Target::Texture { key }, true) => key.clone(),
            _ => continue,
        };
        let Some(&r) = first.get(&key) else {
            first.insert(key, i);
            continue;
        };
        let (fixed, reg) = register(&done[r].img, &done[i].img);
        let ref_id = done[r].asset.id.clone();
        let p = &mut done[i];
        if reg.roll != (0, 0) {
            p.img = fixed;
            // The roll moved the wrap onto the reference's border, where
            // the reference had its edge blend; give this one the same.
            crate::texture::edge_blend(&mut p.img);
            p.fixes.push(format!(
                "rolled by ({}, {}) px into register with {ref_id} (layout correlation {:.2} → {:.2}), then edge-blended (seam ratio {:.2})",
                reg.roll.0,
                reg.roll.1,
                reg.before,
                reg.after,
                arda_tactical::validate::seam_ratio(&p.img)
            ));
        }
        if reg.after < MIN_LAYOUT_NCC {
            p.flags.push(format!(
                "structured layout differs from {ref_id} (correlation {:.2}); the cross-fade will ghost double grout lines",
                reg.after
            ));
        }
    }
}

/// Applies the optional global grade; returns its description.
///
/// # Errors
/// An unreadable reference image or a malformed palette colour.
pub fn grade_all(
    done: &mut [Processed],
    manifest: &Manifest,
    opts: &ImportOptions,
) -> ImportResult<Option<String>> {
    let g = &manifest.grade;
    let reference = opts.reference.clone().or_else(|| {
        g.reference.as_ref().map(|r| {
            let p = PathBuf::from(r);
            match &opts.manifest {
                Some(m) if !p.exists() => m.parent().map_or(p.clone(), |d| d.join(&p)),
                _ => p,
            }
        })
    });
    let (target, what) = if let Some(path) = reference {
        let img = image_io::read(&path)?.rgba;
        let mut acc = Accumulator::default();
        acc.add_image(&img);
        (acc.stats(), format!("reference {}", path.display()))
    } else if !g.palette.is_empty() {
        let colours = g
            .palette
            .iter()
            .map(|c| {
                parse_hex(c)
                    .ok_or_else(|| ImportError::Options(format!("bad palette colour `{c}`")))
            })
            .collect::<ImportResult<Vec<_>>>()?;
        (
            grade::palette_stats(&colours),
            format!("a {}-colour palette", colours.len()),
        )
    } else {
        return Ok(None);
    };
    let strength = opts
        .grade_strength
        .or(g.strength)
        .unwrap_or(DEFAULT_STRENGTH);
    let mut acc = Accumulator::default();
    for p in done.iter() {
        acc.add_image(&p.img);
    }
    let (Some(src), Some(tgt)) = (acc.stats(), target) else {
        return Ok(None);
    };
    let grade = Grade::new(src, tgt, strength);
    for p in done.iter_mut() {
        grade.apply(&mut p.img);
    }
    let m = grade::mean_rgb(&src);
    let after = grade.map([m[0], m[1], m[2], 255]);
    let t = grade::mean_rgb(&tgt);
    Ok(Some(format!(
        "toward {what} at strength {strength:.2}: library mean #{:02x}{:02x}{:02x} → #{:02x}{:02x}{:02x} (target #{:02x}{:02x}{:02x})",
        m[0], m[1], m[2], after[0], after[1], after[2], t[0], t[1], t[2]
    )))
}

fn vocabulary(base: &[Catalog]) -> TagVocabulary {
    let mut v = TagVocabulary::default();
    for c in base {
        let b = &c.vocabulary;
        for (dst, src) in [
            (&mut v.biome, &b.biome),
            (&mut v.culture, &b.culture),
            (&mut v.wealth, &b.wealth),
            (&mut v.function, &b.function),
        ] {
            for t in src {
                if !dst.contains(t) {
                    dst.push(t.clone());
                }
            }
        }
    }
    if base.is_empty() {
        let s = |l: &[&str]| l.iter().map(|x| (*x).to_string()).collect();
        v.biome = s(&["temperate", "boreal", "alpine", "wetland"]);
        v.culture = s(&["human"]);
        v.wealth = s(&["poor", "modest", "wealthy"]);
        v.function = s(vocab::FUNCTIONS);
    }
    v
}

/// Validates one asset on its own (kit completeness is a library matter).
fn asset_issues(p: &Processed, vocab: &TagVocabulary, ppsq: u32) -> Vec<String> {
    let cat = Catalog {
        format_version: FORMAT_VERSION,
        library: "check".into(),
        library_version: "0".into(),
        pixels_per_square: ppsq,
        vocabulary: vocab.clone(),
        assets: vec![p.asset.clone()],
    };
    let images: Images = BTreeMap::from([(p.asset.id.clone(), Ok(p.img.clone()))]);
    validate(&cat, &images, &Thresholds::default())
        .into_iter()
        .filter(|i| i.rule != Rule::WallKit)
        .map(|i| format!("[{}] {}", i.rule.name(), i.message))
        .collect()
}

fn write_png(img: &arda_tactical::Rgba, path: &Path) -> ImportResult<()> {
    img.write_png(path).map_err(ImportError::from)
}

/// Writes the library, validates it alone and stacked over the base, and
/// writes `report.md`, `report.json` and the optional contact sheet.
///
/// # Errors
/// Unwritable outputs.
pub fn write(
    ctx: &Context<'_>,
    mut done: Vec<Processed>,
    skipped: Vec<Skipped>,
    grade: Option<String>,
) -> ImportResult<Report> {
    let out = &ctx.opts.out_dir;
    std::fs::create_dir_all(out).map_err(crate::error::io(out))?;
    let vocab = vocabulary(ctx.base_catalogs);
    let lib = &ctx.manifest.library;
    let name = lib.name.clone().unwrap_or_else(|| "imported".into());
    let version = lib.version.clone().unwrap_or_else(|| "0.1.0".into());
    let mut reports = Vec::new();
    let mut assets = Vec::new();
    for p in &mut done {
        let issues = asset_issues(p, &vocab, ctx.ppsq);
        let status = if issues.is_empty() {
            write_png(&p.img, &out.join(&p.asset.image))?;
            assets.push(p.asset.clone());
            Status::Imported
        } else {
            write_png(
                &p.img,
                &out.join("rejected").join(format!("{}.png", p.asset.id)),
            )?;
            p.flags.extend(
                issues
                    .into_iter()
                    .map(|i| format!("rejected by the validator: {i}")),
            );
            Status::Rejected
        };
        reports.push(AssetReport {
            id: p.asset.id.clone(),
            file: p.file.clone(),
            class: format!("{:?}", p.asset.class).to_lowercase(),
            status,
            fixes: p.fixes.clone(),
            flags: p.flags.clone(),
        });
    }
    let catalog = Catalog {
        format_version: FORMAT_VERSION,
        library: name.clone(),
        library_version: version.clone(),
        pixels_per_square: ctx.ppsq,
        vocabulary: vocab,
        assets,
    };
    let json = catalog::to_json(&catalog)?;
    let cat_path = out.join(library::CATALOG_FILE);
    std::fs::write(&cat_path, json + "\n").map_err(crate::error::io(&cat_path))?;
    let issues = library::check_dir(out, &Thresholds::default())?;
    let (kit, rest): (Vec<_>, Vec<_>) = issues.into_iter().partition(|i| i.rule == Rule::WallKit);
    let mut report = Report {
        library: format!("{name} {version}"),
        assets: reports,
        skipped,
        grade,
        validation: rest.iter().map(ToString::to_string).collect(),
        ..Report::default()
    };
    for i in kit {
        let k = i.asset.trim_start_matches("kit:");
        if ctx.base.covers(&format!("wall:{k}")) {
            report
                .partial_kits
                .push(format!("{i} (the base library supplies them)"));
        } else {
            report.validation.push(i.to_string());
        }
    }
    if let Some(base) = &ctx.opts.base {
        report.stack_validation = Some(stack_issues(out, base));
    }
    report.coverage = coverage(&catalog, ctx.base);
    let md = out.join("report.md");
    std::fs::write(&md, report.to_markdown()).map_err(crate::error::io(&md))?;
    let js = out.join("report.json");
    std::fs::write(&js, serde_json::to_string_pretty(&report)? + "\n")
        .map_err(crate::error::io(&js))?;
    if let Some(sheet) = &ctx.opts.contact_sheet {
        let entries: Vec<contact::Entry<'_>> = done
            .iter()
            .zip(&report.assets)
            .map(|(p, r)| contact::Entry {
                img: &p.img,
                texture: p.target.is_texture(),
                report: r,
            })
            .collect();
        write_png(&contact::sheet(&entries), sheet)?;
    }
    Ok(report)
}

fn stack_issues(out: &Path, base: &Path) -> Vec<String> {
    let mut spec = out.as_os_str().to_owned();
    spec.push(":");
    spec.push(base.as_os_str());
    match Library::load_stack(Path::new(&spec)) {
        Ok(_) => Vec::new(),
        Err(TacticalError::Invalid(v)) => v.iter().map(ToString::to_string).collect(),
        Err(e) => vec![e.to_string()],
    }
}

fn coverage(catalog: &Catalog, base: &Base) -> Coverage {
    let imported = Base::new(std::slice::from_ref(catalog));
    let mut c = Coverage::default();
    for slot in vocab::all_slots() {
        if imported.covers(&slot) {
            c.imported.push(slot);
        } else if base.covers(&slot) {
            c.fallback.push(slot);
        } else {
            c.missing.push(slot);
        }
    }
    c
}
