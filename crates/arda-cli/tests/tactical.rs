//! `arda tactical …` surface tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::Command;

fn arda(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_arda"))
        .args(args)
        .output()
        .unwrap()
}

fn placeholder() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    dir.to_string_lossy().into_owned()
}

fn temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("arda-cli-tactical-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn validate_accepts_the_committed_placeholders() {
    let out = arda(&["tactical", "validate", &placeholder()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("ok (255 assets"));
}

#[test]
fn validate_names_the_asset_and_rule_on_failure() {
    let dir = temp("bad");
    let catalog =
        std::fs::read_to_string(PathBuf::from(placeholder()).join("catalog.json")).unwrap();
    // Drop every image: each asset must fail `image_missing`.
    std::fs::write(dir.join("catalog.json"), catalog).unwrap();
    let out = arda(&["tactical", "validate", dir.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("prop.barrel: [image_missing]"), "{err}");
}

#[test]
fn render_writes_a_png_of_the_requested_size() {
    let dir = temp("render");
    let png = dir.join("junctions.png");
    let out = arda(&[
        "tactical",
        "render",
        "--layout",
        "wall_junctions",
        "--library",
        &placeholder(),
        "--ppsq",
        "16",
        "--grid",
        "--out",
        png.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let bytes = std::fs::read(&png).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
    let (w, h) = (
        u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
        u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
    );
    assert_eq!((w, h), (16 * 16, 9 * 16));
}

#[test]
fn render_rejects_unknown_layouts() {
    let out = arda(&[
        "tactical",
        "render",
        "--layout",
        "atlantis",
        "--library",
        &placeholder(),
    ]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("neither a built-in layout"));
}
