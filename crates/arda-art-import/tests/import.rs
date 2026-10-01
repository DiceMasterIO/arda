//! End-to-end import of a synthetic raw set: the output validates on its
//! own and stacked over the placeholders, defects are fixed or flagged,
//! and two runs are byte-identical.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

mod fixtures;

use arda_art_import::report::{Report, Status};
use arda_art_import::{import, ImportOptions};
use arda_tactical::validate::seam_ratio;
use arda_tactical::{Library, Rgba};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("art-import")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn placeholder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder")
}

fn run(root: &Path) -> Report {
    let raw = root.join("raw");
    fixtures::write_raw_set(&raw);
    let manifest = root.join("import.toml");
    std::fs::write(&manifest, fixtures::MANIFEST).unwrap();
    import(&ImportOptions {
        raw_dir: raw,
        out_dir: root.join("lib"),
        manifest: Some(manifest),
        base: Some(placeholder()),
        contact_sheet: Some(root.join("sheet.png")),
        ..ImportOptions::default()
    })
    .unwrap()
}

/// One shared import for the assertions that only read its output.
fn shared() -> &'static (PathBuf, Report) {
    static RUN: OnceLock<(PathBuf, Report)> = OnceLock::new();
    RUN.get_or_init(|| {
        let root = scratch("shared");
        let report = run(&root);
        (root, report)
    })
}

fn asset<'a>(r: &'a Report, id: &str) -> &'a arda_art_import::report::AssetReport {
    r.assets.iter().find(|a| a.id == id).unwrap_or_else(|| {
        panic!(
            "{id} not in {:?}",
            r.assets.iter().map(|a| &a.id).collect::<Vec<_>>()
        )
    })
}

fn image(root: &Path, rel: &str) -> Rgba {
    Rgba::read_png(&root.join("lib").join(rel)).unwrap()
}

#[test]
fn the_output_passes_the_validator_alone_and_stacked() {
    let (root, r) = shared();
    assert!(r.valid(), "{}", r.to_markdown());
    assert_eq!(r.stack_validation.as_deref(), Some(&[][..]));
    assert!(
        r.assets.iter().all(|a| a.status == Status::Imported),
        "{}",
        r.to_markdown()
    );
    // The stone kit holds only a corner: partial, completed by the base.
    assert!(r.partial_kits.iter().any(|k| k.contains("kit:stone")));
    let mut spec = root.join("lib").into_os_string();
    spec.push(":");
    spec.push(placeholder());
    let lib = Library::load_stack(Path::new(&spec)).unwrap();
    assert!(lib
        .asset("prop.barrel")
        .unwrap()
        .provenance
        .contains("seed 77"));
    assert!(
        lib.asset("prop.chest").is_some(),
        "falls back to the placeholder"
    );
    assert!(r.coverage.fallback.contains(&"prop.chest".to_string()));
    assert!(r.coverage.imported.contains(&"ground:grass".to_string()));
}

#[test]
fn the_baked_shadow_and_halo_are_removed() {
    let (root, r) = shared();
    let a = asset(r, "prop.barrel");
    assert!(
        a.fixes
            .iter()
            .any(|f| f.contains("stripped a baked drop shadow") && f.contains("SE")),
        "{:?}",
        a.fixes
    );
    let img = image(root, "props/prop.barrel.png");
    assert_eq!((img.width, img.height), (128, 128));
    // With the shadow gone the silhouette is a centred disc again.
    let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
    let (mut grey_rim, mut halo) = (0, 0);
    for y in 0..128 {
        for x in 0..128 {
            let p = img.get(x, y);
            let a = f64::from(p[3]) / 255.0;
            sx += f64::from(x) * a;
            sy += f64::from(y) * a;
            n += a;
            let sat = p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
            if p[3] > 0 && sat < 12 && p[0] > 90 {
                grey_rim += 1; // shadow grey
            }
            if p[3] > 0 && p[0] > 230 && p[1] > 220 {
                halo += 1; // pale glow
            }
        }
    }
    let (cx, cy) = (sx / n, sy / n);
    assert!(
        (cx - 63.5).abs() < 2.0 && (cy - 63.5).abs() < 2.0,
        "centroid {cx:.1},{cy:.1}"
    );
    assert!(grey_rim < 20, "{grey_rim} shadow-grey px left");
    assert!(halo < 10, "{halo} halo px left");
}

#[test]
fn a_shadow_baked_into_alpha_is_stripped() {
    let (root, r) = shared();
    let a = asset(r, "veg.bush");
    assert!(
        a.fixes.iter().any(|f| f.contains("stripped")),
        "{:?} / {:?}",
        a.fixes,
        a.flags
    );
    let img = image(root, "vegetation/veg.bush.png");
    let dark = img
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[3] > 0 && p[1] < 50)
        .count();
    assert!(dark < 30, "{dark} dark px left");
}

#[test]
fn textures_are_resized_tileable_and_registered() {
    let (root, r) = shared();
    for id in ["ground.grass.0", "ground.grass.1"] {
        let img = image(root, &format!("ground/{id}.png"));
        assert_eq!(
            (img.width, img.height),
            (256, 256),
            "{id} keeps the 2x2 placeholder footprint"
        );
        assert!(seam_ratio(&img) < 2.0, "{id}");
        assert!(asset(r, id)
            .fixes
            .iter()
            .any(|f| f.contains("offset-and-blend")));
    }
    let img = image(root, "ground/ground.cobbles.1.png");
    assert_eq!((img.width, img.height), (512, 512));
    let c1 = asset(r, "ground.cobbles.1");
    assert!(
        c1.fixes.iter().any(|f| f.contains("into register")),
        "{:?} {:?}",
        c1.fixes,
        c1.flags
    );
    assert!(c1.flags.is_empty(), "{:?}", c1.flags);
}

#[test]
fn walls_are_turned_to_their_canonical_arms() {
    let (_, r) = shared();
    let a = asset(r, "wall.stone.corner");
    assert!(
        a.fixes.iter().any(|f| f.contains("turned 180°")),
        "{:?} {:?}",
        a.fixes,
        a.flags
    );
}

#[test]
fn names_metadata_and_unknowns_are_reported() {
    let (root, r) = shared();
    let anvil = asset(r, "prop.anvill");
    assert!(
        anvil
            .flags
            .iter()
            .any(|f| f.contains("did you mean `anvil`")),
        "{:?}",
        anvil.flags
    );
    assert!(r.skipped.iter().any(|s| s.file == "ComfyUI_00001_.png"));
    let cat = std::fs::read_to_string(root.join("lib/catalog.json")).unwrap();
    let cat = arda_tactical::catalog::parse(&cat).unwrap();
    let crate_ = cat.assets.iter().find(|a| a.id == "prop.crate").unwrap();
    // Provenance from the PNG's ComfyUI graph; metadata from the placeholder record.
    assert!(
        crate_.provenance.contains("seed 4242"),
        "{}",
        crate_.provenance
    );
    assert!(
        crate_.provenance.contains("flux1-schnell"),
        "{}",
        crate_.provenance
    );
    assert_eq!(crate_.licence, "CC0-1.0");
    assert_eq!(crate_.height_ft, 3);
    assert!(crate_.blocks_movement);
    assert!(r.grade.as_deref().unwrap_or("").contains("palette"));
    assert!(root.join("sheet.png").exists());
    assert!(root.join("lib/report.md").exists());
}

#[test]
fn two_runs_are_byte_identical() {
    let (a, b) = (scratch("det-a"), scratch("det-b"));
    run(&a);
    run(&b);
    let mut files = Vec::new();
    collect(&a.join("lib"), &mut files);
    assert!(files.len() > 8);
    for f in files {
        let rel = f.strip_prefix(&a).unwrap();
        assert_eq!(
            std::fs::read(&f).unwrap(),
            std::fs::read(b.join(rel)).unwrap(),
            "{}",
            rel.display()
        );
    }
    assert_eq!(
        std::fs::read(a.join("sheet.png")).unwrap(),
        std::fs::read(b.join("sheet.png")).unwrap()
    );
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        } else {
            out.push(p);
        }
    }
}
