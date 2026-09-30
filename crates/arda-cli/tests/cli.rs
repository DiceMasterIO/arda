//! CLI surface tests (`mockup/01`, `mockup/03`).
//!
//! Integration tests compile as their own crate; `code-prefs.md` §Q1 permits
//! unwrap in tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;
use std::sync::LazyLock;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_arda")
}

fn png_dimensions(path: &std::path::Path) -> (u32, u32) {
    use std::io::Read;
    let mut header = [0u8; 24];
    std::fs::File::open(path)
        .unwrap()
        .read_exact(&mut header)
        .unwrap();
    (
        u32::from_be_bytes(header[16..20].try_into().unwrap()),
        u32::from_be_bytes(header[20..24].try_into().unwrap()),
    )
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("arda-cli-{}-{tag}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        Self(p)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn generate_micro(dir: &std::path::Path) -> std::process::Output {
    Command::new(bin())
        .args(["generate", "--seed", "42", "--micro", "--out"])
        .arg(dir)
        .output()
        .unwrap()
}

struct MicroFixture {
    output: std::process::Output,
    files: Vec<(std::path::PathBuf, Vec<u8>)>,
}

fn micro_fixture() -> &'static MicroFixture {
    static FIXTURE: LazyLock<MicroFixture> = LazyLock::new(|| {
        let dir = TempDir::new("fixture");
        let output = generate_micro(dir.path());
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        fn snapshot(
            root: &std::path::Path,
            dir: &std::path::Path,
            files: &mut Vec<(std::path::PathBuf, Vec<u8>)>,
        ) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    snapshot(root, &path, files);
                } else {
                    files.push((
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        std::fs::read(path).unwrap(),
                    ));
                }
            }
        }

        let mut files = Vec::new();
        snapshot(dir.path(), dir.path(), &mut files);
        MicroFixture { output, files }
    });
    &FIXTURE
}

fn micro_world(tag: &str) -> TempDir {
    let dir = TempDir::new(tag);
    // Share immutable file bytes, so destructive tests keep their own world and
    // every on-disk directory is still cleaned up when its owner is dropped.
    for (relative_path, bytes) in &micro_fixture().files {
        let path = dir.path().join(relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    dir
}

#[test]
fn generate_writes_a_world_and_exits_zero() {
    let dir = micro_world("gen");
    let out = &micro_fixture().output;
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.path().join("world.json").is_file());

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("seed 42"), "stdout was: {stdout}");
    assert!(stdout.contains("done"), "stdout was: {stdout}");
}

#[test]
fn generate_into_an_occupied_directory_exits_non_zero() {
    let dir = micro_world("occupied");
    let out = generate_micro(dir.path());
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("not empty"));
}

#[test]
fn invalid_config_exits_non_zero_naming_the_field() {
    let dir = TempDir::new("badcfg");
    let out = Command::new(bin())
        .args(["generate", "--seed", "1", "--size", "10x10", "--out"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("size"));
}

#[test]
fn export_writes_a_png_for_an_area() {
    let world = micro_world("exp-world");
    let out = TempDir::new("exp-out");

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--area", "1,1", "--format", "png", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(
        res.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&res.stderr)
    );
    assert!(out.path().join("area_01_01.png").is_file());
    assert_eq!(
        png_dimensions(&out.path().join("area_01_01.png")),
        (8192, 8192)
    );

    for quality in ["512", "513"] {
        let result = Command::new(bin())
            .args(["export", "--world"])
            .arg(world.path())
            .args(["--area", "1,1", "--quality", quality, "--out"])
            .arg(out.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let side = quality.parse().unwrap();
        assert_eq!(
            png_dimensions(&out.path().join("area_01_01.png")),
            (side, side)
        );
    }
}

#[test]
fn explicit_atlas_routes_area_overview_and_detail_pngs() {
    let world = micro_world("atlas-world");
    for (tag, args, file, dimensions) in [
        (
            "atlas-area",
            vec!["--area", "1,1", "--quality", "513", "--style", "atlas"],
            "area_01_01.png",
            (513, 513),
        ),
        (
            "atlas-overview",
            vec!["--overview", "--quality", "512", "--style", "atlas"],
            "overview.png",
            (256, 512),
        ),
        (
            "atlas-detail",
            vec!["--area", "1,1", "--detail", "--style", "atlas"],
            "area_01_01.png",
            (4096, 4096),
        ),
    ] {
        let out = TempDir::new(tag);
        let mut command = Command::new(bin());
        command.args(["export", "--world"]).arg(world.path());
        command.args(args).arg("--out").arg(out.path());
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(png_dimensions(&out.path().join(file)), dimensions);
    }
}

#[test]
fn classic_style_is_byte_identical_to_omitted_style() {
    let world = micro_world("classic-style-world");
    let omitted = TempDir::new("classic-omitted");
    let explicit = TempDir::new("classic-explicit");
    for (out, style) in [(&omitted, None), (&explicit, Some("classic"))] {
        let mut command = Command::new(bin());
        command.args(["export", "--world"]).arg(world.path()).args([
            "--area",
            "1,1",
            "--quality",
            "513",
        ]);
        if let Some(style) = style {
            command.args(["--style", style]);
        }
        let result = command.arg("--out").arg(out.path()).output().unwrap();
        assert!(result.status.success());
    }
    assert_eq!(
        std::fs::read(omitted.path().join("area_01_01.png")).unwrap(),
        std::fs::read(explicit.path().join("area_01_01.png")).unwrap(),
    );
}

#[test]
fn explicit_style_refusals_create_no_output_directory() {
    let missing = TempDir::new("style-refusal-parent");
    let cases = [
        vec!["--format", "json", "--style", "classic"],
        vec!["--block", "0,0,0,0", "--style", "atlas"],
    ];
    for (index, args) in cases.into_iter().enumerate() {
        let out = missing.path().join(format!("case-{index}"));
        let mut command = Command::new(bin());
        command.args(["export", "--world", "missing-world"]);
        command.args(args).arg("--out").arg(&out);
        let result = command.output().unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr)
            .contains("--style is valid only for area or overview PNG exports"));
        assert!(!out.exists());
    }
}

#[test]
fn export_writes_json_when_asked() {
    let world = micro_world("exp-json-world");
    let out = TempDir::new("exp-json-out");

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--area", "0,1", "--format", "json", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(res.status.success());
    let text = std::fs::read_to_string(out.path().join("area_00_01.json")).unwrap();
    assert!(text.contains("\"schema_version\""));
}

#[test]
fn export_refuses_a_partial_world() {
    let world = micro_world("exp-partial");
    let out = TempDir::new("exp-partial-out");
    std::fs::remove_file(world.path().join("world.json")).unwrap();

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--area", "1,1", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(!res.status.success());
}

#[test]
fn preview_generates_a_world_and_one_overview_image() {
    let dir = TempDir::new("preview");
    let out = Command::new(bin())
        .args(["preview", "--seed", "42", "--micro", "--px", "16", "--out"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.path().join("world/world.json").is_file());
    let png = dir.path().join("overview.png");
    assert!(png.is_file(), "no overview.png written");

    // 2 areas wide, 4 high, 16 px each.
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
    );
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    assert_eq!((width, height), (2 * 16, 4 * 16));
}

#[test]
fn preview_generates_a_world_and_one_atlas_overview_image() {
    let dir = TempDir::new("preview-atlas");
    let out = Command::new(bin())
        .args([
            "preview",
            "--seed",
            "42",
            "--micro",
            "--quality",
            "512",
            "--style",
            "atlas",
            "--out",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dir.path().join("world/world.json").is_file());
    let png = dir.path().join("overview.png");
    assert!(png.is_file(), "no overview.png written");
    assert_eq!(png_dimensions(&png), (256, 512));
}

#[test]
fn export_overview_renders_an_existing_world() {
    let world = micro_world("ov-world");
    let out = TempDir::new("ov-out");

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--overview", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(
        res.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&res.stderr)
    );
    assert!(out.path().join("overview.png").is_file());
    assert_eq!(
        png_dimensions(&out.path().join("overview.png")),
        (4096, 8192)
    );
}

#[test]
fn export_renders_a_tactical_block() {
    // The wave-function-collapse layer had no command at all until now.
    //
    // Which cells carry a block depends on where the land falls, so this
    // walks the stride rather than pinning one coordinate — an earlier
    // version hard-coded a cell that later became sea.
    let world = micro_world("blk-world");
    let out = TempDir::new("blk-out");

    let mut rendered = None;
    'search: for ay in 0..4 {
        for cy in (0..512).step_by(64) {
            for cx in (0..512).step_by(64) {
                let spec = format!("0,{ay},{cx},{cy}");
                let res = Command::new(bin())
                    .args(["export", "--world"])
                    .arg(world.path())
                    .args(["--block", &spec, "--out"])
                    .arg(out.path())
                    .output()
                    .unwrap();
                if res.status.success() {
                    rendered = Some(format!("block_00_{ay:02}_{cx:03}_{cy:03}.png"));
                    break 'search;
                }
            }
        }
    }
    let name = rendered.expect("no land cell in the micro world carried a block");
    assert!(out.path().join(&name).is_file(), "{name} was not written");
}

#[test]
fn export_block_names_the_stride_when_there_is_none() {
    let world = micro_world("blk-miss");
    let out = TempDir::new("blk-miss-out");

    let res = Command::new(bin())
        .args(["export", "--world"])
        .arg(world.path())
        .args(["--block", "0,1,7,7", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("64-cell stride"));
}
