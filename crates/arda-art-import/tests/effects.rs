//! Effect art through the importer: alpha survives, light-on-black sources
//! get alpha from brightness, textures wrap, ids are numbered, and the
//! output validates alone and stacked over the placeholders.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use arda_art_import::report::Status;
use arda_art_import::{import, ImportOptions};
use arda_tactical::catalog::AssetClass;
use arda_tactical::validate::seam_ratio_alpha;
use arda_tactical::{Library, Rgba};
use std::path::{Path, PathBuf};

fn root(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("art-import-effects")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("raw")).unwrap();
    dir
}

fn placeholder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder")
}

/// A soft cloud; `alpha` false makes it opaque.
fn cloud(side: u32, alpha: bool, black_edge: bool) -> Rgba {
    let mut img = Rgba::new(side, side);
    for y in 0..side {
        for x in 0..side {
            let (dx, dy) = (x as f32 / side as f32 - 0.5, y as f32 / side as f32 - 0.5);
            let mut v = (1.0 - (dx * dx + dy * dy).sqrt() * 1.8).clamp(0.0, 1.0);
            if !black_edge {
                v = 0.25 + 0.75 * v;
            }
            let c = (v * 240.0) as u8;
            let a = if alpha { (v * 230.0) as u8 + 10 } else { 255 };
            img.set(x, y, [c, c / 2 + 10, 20, a]);
        }
    }
    img
}

fn run(name: &str, manifest: &str) -> (PathBuf, arda_art_import::report::Report) {
    let root = root(name);
    let raw = root.join("raw");
    cloud(300, true, false)
        .write_png(&raw.join("effect.frost.a.png"))
        .unwrap();
    cloud(280, true, false)
        .write_png(&raw.join("effect.frost.b.png"))
        .unwrap();
    // Light on black, opaque: alpha must come from brightness.
    cloud(256, false, true)
        .write_png(&raw.join("effect.fire.png"))
        .unwrap();
    cloud(256, true, false)
        .write_png(&raw.join("effect.scorch_mark.png"))
        .unwrap();
    std::fs::write(root.join("import.toml"), manifest).unwrap();
    let report = import(&ImportOptions {
        raw_dir: raw,
        out_dir: root.join("lib"),
        manifest: Some(root.join("import.toml")),
        base: Some(placeholder()),
        ..ImportOptions::default()
    })
    .unwrap();
    (root, report)
}

const MANIFEST: &str = r#"
[library]
name = "effects-test"
licence = "CC0-1.0"
tool = "test"
model = "synthetic"
"#;

#[test]
fn effects_keep_alpha_and_validate_alone_and_stacked() {
    let (root, r) = run("keep", MANIFEST);
    assert!(r.valid(), "{}", r.to_markdown());
    assert!(
        r.assets.iter().all(|a| a.status == Status::Imported),
        "{}",
        r.to_markdown()
    );
    assert_eq!(r.stack_validation.as_deref(), Some(&[][..]));
    let ids: Vec<&str> = r.assets.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "effect.fire.0",
            "effect.frost.0",
            "effect.frost.1",
            "effect.scorch_mark.0"
        ]
    );
    let lib = Library::load(&root.join("lib")).unwrap();
    for id in ids {
        let a = lib.asset(id).unwrap();
        assert_eq!(a.class, AssetClass::Effect, "{id}");
        assert!(
            !a.blocks_movement && !a.blocks_sight && !a.casts_shadow,
            "{id}"
        );
        let img = lib.image(id).unwrap();
        let px = img.data.as_chunks::<4>().0;
        if id.starts_with("effect.scorch") {
            assert!(px.iter().any(|p| p[3] == 0), "{id} has clear pixels");
        }
        assert!(
            px.iter().any(|p| p[3] > 0 && p[3] < 255),
            "{id} keeps soft alpha"
        );
    }
    assert!(lib.asset("effect.frost.0").unwrap().tileable);
    assert!(!lib.asset("effect.scorch_mark.0").unwrap().tileable);
    assert!(seam_ratio_alpha(lib.image("effect.frost.0").unwrap()) <= 2.0);
    // Stacked: the imported frost set (2 variants) replaces the placeholder's 3.
    let mut spec = root.join("lib").into_os_string();
    spec.push(":");
    spec.push(placeholder());
    let stack = Library::load_stack(Path::new(&spec)).unwrap();
    let frost = stack
        .catalog
        .assets
        .iter()
        .filter(|a| a.id.starts_with("effect.frost."))
        .count();
    assert_eq!(frost, 2);
    assert!(
        stack.asset("effect.fog.0").is_some(),
        "other effects fall back"
    );
}

#[test]
fn the_luminance_key_and_border_detection_agree() {
    let (_, auto) = run("auto", MANIFEST);
    let fire = auto
        .assets
        .iter()
        .find(|a| a.id == "effect.fire.0")
        .unwrap();
    assert!(
        fire.fixes.iter().any(|f| f.contains("black border")),
        "{:?}",
        fire.fixes
    );
    let forced = format!("{MANIFEST}\n[[asset]]\nid = \"effect.fire\"\nalpha = \"luminance\"\n");
    let (root_a, _) = run("auto2", MANIFEST);
    let (root_b, b) = run("forced", &forced);
    let fire = b.assets.iter().find(|a| a.id == "effect.fire.0").unwrap();
    assert!(fire.fixes.iter().any(|f| f.contains("alpha = luminance")));
    let png = |r: &Path| std::fs::read(r.join("lib/effects/effect.fire.0.png")).unwrap();
    assert_eq!(png(&root_a), png(&root_b), "same alpha either way");
}

#[test]
fn two_runs_are_byte_identical() {
    let (a, _) = run("det-a", MANIFEST);
    let (b, _) = run("det-b", MANIFEST);
    for f in [
        "effects/effect.frost.0.png",
        "effects/effect.scorch_mark.0.png",
        "catalog.json",
    ] {
        assert_eq!(
            std::fs::read(a.join("lib").join(f)).unwrap(),
            std::fs::read(b.join("lib").join(f)).unwrap(),
            "{f}"
        );
    }
}
