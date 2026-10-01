#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn style_parses_and_omission_defaults_to_classic() {
    for (text, expected) in [
        ("classic", MapStyle::Classic),
        ("atlas", MapStyle::Atlas),
        ("atlas-oblique", MapStyle::Atlas),
    ] {
        let cli = Cli::try_parse_from([
            "arda", "export", "--world", "saved", "--out", "exports", "--style", text,
        ])
        .unwrap();
        let Command::Export {
            style,
            detail,
            quality,
            ..
        } = cli.command
        else {
            panic!("expected export")
        };
        assert_eq!(
            ImageOptions {
                detail,
                quality,
                style
            }
            .style(),
            expected
        );
    }
    let cli =
        Cli::try_parse_from(["arda", "export", "--world", "saved", "--out", "exports"]).unwrap();
    let Command::Export {
        style,
        detail,
        quality,
        ..
    } = cli.command
    else {
        panic!("expected export")
    };
    assert!(style.is_none());
    assert_eq!(
        ImageOptions {
            detail,
            quality,
            style
        }
        .style(),
        MapStyle::Classic
    );
}

#[test]
fn fine_generation_is_explicit_and_legacy_remains_default() {
    let legacy = Cli::try_parse_from([
        "arda", "generate", "--seed", "42", "--micro", "--out", "world",
    ])
    .unwrap();
    assert!(matches!(
        legacy.command,
        Command::Generate {
            terrain: Terrain::Legacy,
            ..
        }
    ));
    let fine = Cli::try_parse_from([
        "arda",
        "generate",
        "--seed",
        "42",
        "--micro",
        "--out",
        "world",
        "--terrain",
        "fine",
        "--fine-ram-bytes",
        "123456",
    ])
    .unwrap();
    assert!(matches!(
        fine.command,
        Command::Generate {
            terrain: Terrain::Fine,
            fine_ram_bytes: Some(123456),
            ..
        }
    ));
}

#[test]
fn recipe_is_a_fine_option_from_4_to_7() {
    let parse = |recipe: &str| {
        Cli::try_parse_from([
            "arda",
            "generate",
            "--seed",
            "42",
            "--micro",
            "--out",
            "world",
            "--terrain",
            "fine",
            "--recipe",
            recipe,
        ])
    };
    assert!(matches!(
        parse("5").unwrap().command,
        Command::Generate {
            recipe: Some(5),
            ..
        }
    ));
    assert!(parse("3").is_err());
    assert!(parse("7").is_ok());
    assert!(parse("8").is_err());
    let default = Cli::try_parse_from([
        "arda",
        "generate",
        "--seed",
        "42",
        "--out",
        "world",
        "--terrain",
        "fine",
    ])
    .unwrap();
    assert!(matches!(
        default.command,
        Command::Generate { recipe: None, .. }
    ));
    // Legacy terrain has no recipe.
    let legacy = run_generate(
        42,
        "500x1000",
        true,
        Path::new("unused-world"),
        Terrain::Legacy,
        FineOptions {
            recipe: Some(5),
            ram_bytes: None,
            file_bytes: None,
            latitude: None,
        },
    );
    assert!(legacy.unwrap_err().to_string().contains("--recipe"));
}

#[test]
fn latitude_bands_parse_and_recipe_7_is_accepted() {
    assert_eq!(parse_latitude("15,35").unwrap(), LatitudeBand::new(15, 35));
    assert_eq!(
        parse_latitude(" -40, -20").unwrap(),
        LatitudeBand::new(-40, -20)
    );
    assert!(parse_latitude("15").is_err());
    assert!(parse_latitude("a,b").is_err());
    let cli = Cli::try_parse_from([
        "arda",
        "generate",
        "--seed",
        "74",
        "--micro",
        "--out",
        "w",
        "--terrain",
        "fine",
        "--recipe",
        "7",
        "--latitude",
        "15,35",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Command::Generate {
            recipe: Some(7),
            ..
        }
    ));
    assert!(Cli::try_parse_from([
        "arda", "generate", "--seed", "1", "--out", "w", "--recipe", "8",
    ])
    .is_err());
}

#[test]
fn unknown_style_is_a_parse_error() {
    assert!(Cli::try_parse_from([
        "arda", "export", "--world", "saved", "--out", "exports", "--style", "painted",
    ])
    .is_err());
}

#[test]
fn explicit_style_rejects_json_and_blocks_before_io() {
    for (format, block) in [
        (Format::Json, None),
        (Format::Json, Some("0,0,0,0")),
        (Format::Png, Some("0,0,0,0")),
    ] {
        let result = run_export(
            Path::new("missing-world"),
            "0,0",
            format,
            false,
            block,
            Path::new("must-not-exist"),
            ImageOptions {
                detail: false,
                quality: None,
                style: Some(Style::Atlas),
            },
        );
        assert!(result.is_err_and(|error| {
            error.to_string() == "--style is valid only for area or overview PNG exports"
        }));
    }
    assert!(ImageOptions::default()
        .validate(Format::Json, false, None)
        .is_ok());
}

#[test]
fn preview_legacy_px_conflicts_with_any_explicit_style() {
    for style in ["classic", "atlas"] {
        assert!(Cli::try_parse_from([
            "arda", "preview", "--seed", "42", "--out", "unused", "--px", "16", "--style", style,
        ])
        .is_err());
    }
    assert!(Cli::try_parse_from([
        "arda", "preview", "--seed", "42", "--out", "unused", "--px", "16",
    ])
    .is_ok());
}

#[test]
fn detail_accepts_atlas_but_still_rejects_quality_overview_and_blocks() {
    assert!(Cli::try_parse_from([
        "arda", "export", "--world", "saved", "--out", "exports", "--detail", "--style", "atlas",
    ])
    .is_ok());
    for extra in [
        vec!["--quality", "4k"],
        vec!["--overview"],
        vec!["--block", "0,0,0,0"],
    ] {
        let mut args = vec![
            "arda", "export", "--world", "saved", "--out", "exports", "--detail", "--style",
            "atlas",
        ];
        args.extend(extra);
        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn detail_conflicts_with_overview_and_blocks() {
    for extra in [vec!["--overview"], vec!["--block", "0,0,0,0"]] {
        let mut args = vec![
            "arda", "export", "--world", "missing", "--out", "unused", "--detail",
        ];
        args.extend(extra);
        assert!(Cli::try_parse_from(args).is_err());
    }
}

#[test]
fn detail_json_reports_the_invalid_mode_before_reading_the_world() {
    let error = run_export(
        Path::new("missing-world"),
        "0,0",
        Format::Json,
        false,
        None,
        Path::new("unused-export"),
        ImageOptions {
            detail: true,
            quality: None,
            style: None,
        },
    );
    assert!(error.is_err_and(|e| e.to_string() == "--detail is valid only for area PNG exports"));
}

#[test]
fn detail_is_an_explicit_area_png_option() {
    let cli = Cli::try_parse_from([
        "arda", "export", "--world", "saved", "--out", "exports", "--detail",
    ]);
    assert!(matches!(
        cli,
        Ok(Cli {
            command: Command::Export {
                detail: true,
                format: Format::Png,
                ..
            }
        })
    ));
}

#[test]
fn png_quality_defaults_to_8k_for_area_and_overview() {
    for mode in [vec!["--area", "1,1"], vec!["--overview"]] {
        let mut args = vec!["arda", "export", "--world", "saved", "--out", "exports"];
        args.extend(mode);
        let Cli {
            command: Command::Export {
                detail, quality, ..
            },
        } = Cli::try_parse_from(args).unwrap()
        else {
            panic!("expected export")
        };
        assert_eq!(
            ImageOptions {
                detail,
                quality,
                style: None,
            }
            .quality()
            .pixels(),
            8192
        );
    }
}

#[test]
fn quality_accepts_pixels_and_k_suffixes_and_rejects_invalid_values() {
    for (text, pixels) in [("512", 512), ("513", 513), ("8k", 8192), ("32k", 32768)] {
        let cli = Cli::try_parse_from([
            "arda",
            "export",
            "--world",
            "saved",
            "--out",
            "exports",
            "--quality",
            text,
        ])
        .unwrap();
        let Command::Export {
            quality: Some(quality),
            ..
        } = cli.command
        else {
            panic!("missing quality")
        };
        assert_eq!(quality.pixels(), pixels);
    }
    for text in ["511", "32769", "33k", "bogus"] {
        assert!(Cli::try_parse_from([
            "arda",
            "export",
            "--world",
            "saved",
            "--out",
            "exports",
            "--quality",
            text,
        ])
        .is_err());
    }
}

#[test]
fn explicit_quality_rejects_non_png_modes_before_reading_the_world() {
    let image = ImageOptions {
        detail: false,
        quality: Some(ImageQuality::DEFAULT),
        style: None,
    };
    for (format, overview, block) in [
        (Format::Json, false, None),
        (Format::Json, true, None),
        (Format::Png, false, Some("0,0,0,0")),
    ] {
        let result = run_export(
            Path::new("missing-world"),
            "0,0",
            format,
            overview,
            block,
            Path::new("unused-export"),
            image,
        );
        assert!(result.is_err_and(
            |e| e.to_string() == "--quality is valid only for area or overview PNG exports"
        ));
    }
    assert!(ImageOptions::default()
        .validate(Format::Json, false, None)
        .is_ok());
}

#[test]
fn explicit_quality_conflicts_with_legacy_resolution_flags_and_blocks() {
    for extra in [vec!["--detail"], vec!["--block", "0,0,0,0"]] {
        let mut args = vec![
            "arda",
            "export",
            "--world",
            "saved",
            "--out",
            "exports",
            "--quality",
            "8k",
        ];
        args.extend(extra);
        assert!(Cli::try_parse_from(args).is_err());
    }
    assert!(Cli::try_parse_from([
        "arda",
        "preview",
        "--seed",
        "42",
        "--out",
        "exports",
        "--quality",
        "8k",
        "--px",
        "16",
    ])
    .is_err());
}
