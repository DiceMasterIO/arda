//! One failing fixture per validator rule (goal 60).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use crate::catalog::{
    Anchor, Asset, AssetClass, Catalog, Footprint, Layer, TagVocabulary, WallPiece, WallRole,
};
use crate::noise::fbm;
use crate::raster::Rgba;
use crate::validate::{seam_ratio, validate, Images, Issue, Rule, Thresholds};

const PPSQ: u32 = 16;

fn asset(id: &str, class: AssetClass, layer: Layer) -> Asset {
    Asset {
        id: id.into(),
        class,
        image: format!("{id}.png"),
        footprint: Footprint { w: 1, h: 1 },
        anchor: Some(Anchor { x: 0.5, y: 0.5 }),
        rotations: vec![0, 90],
        mirror: false,
        tags: crate::catalog::Tags::default(),
        placement: crate::catalog::PlacementRules::default(),
        blocks_sight: false,
        blocks_movement: true,
        difficult_terrain: false,
        cover: crate::catalog::Cover::Half,
        light: None,
        layer,
        z: 0,
        casts_shadow: true,
        height_ft: 3,
        tileable: false,
        ground: None,
        wall: None,
        provenance: "test fixture".into(),
        licence: "Apache-2.0".into(),
    }
}

fn disc() -> Rgba {
    let mut img = Rgba::new(PPSQ, PPSQ);
    for y in 0..PPSQ {
        for x in 0..PPSQ {
            let (dx, dy) = (x as i32 - 8, y as i32 - 8);
            let d2 = dx * dx + dy * dy;
            let a = if d2 < 36 {
                255
            } else if d2 < 49 {
                128
            } else {
                0
            };
            img.set(x, y, [120, 80, 40, a]);
        }
    }
    img
}

fn texture() -> Rgba {
    let mut img = Rgba::new(PPSQ, PPSQ);
    for y in 0..PPSQ {
        for x in 0..PPSQ {
            let v = fbm(3, x as f32 / 8.0, y as f32 / 8.0, 2, Some(2));
            let g = (60.0 + v * 120.0) as u8;
            img.set(x, y, [g / 2, g, g / 3, 255]);
        }
    }
    img
}

/// A catalogue that passes every rule, plus its images.
fn good() -> (Catalog, Images) {
    let mut assets = vec![asset("prop.barrel", AssetClass::Prop, Layer::Prop)];
    let mut grass = asset("ground.grass", AssetClass::Ground, Layer::Ground);
    grass.tileable = true;
    grass.ground = Some("grass".into());
    assets.push(grass);
    for role in WallRole::REQUIRED {
        let mut w = asset(
            &format!("wall.stone.{role:?}"),
            AssetClass::Wall,
            Layer::Wall,
        );
        w.wall = Some(WallPiece {
            kit: "stone".into(),
            role,
        });
        assets.push(w);
    }
    assets[0].tags.function = vec!["warehouse".into()];
    let images = assets
        .iter()
        .map(|a| {
            let img = if a.class.is_texture() {
                texture()
            } else {
                disc()
            };
            (a.id.clone(), Ok(img))
        })
        .collect();
    let catalog = Catalog {
        format_version: 1,
        library: "fixture".into(),
        library_version: "1".into(),
        pixels_per_square: PPSQ,
        vocabulary: TagVocabulary {
            function: vec!["warehouse".into()],
            ..TagVocabulary::default()
        },
        assets,
    };
    (catalog, images)
}

fn run(c: &Catalog, i: &Images) -> Vec<Issue> {
    validate(c, i, &Thresholds::default())
}

/// Asserts exactly one issue, naming `asset` and `rule`.
fn assert_only(issues: &[Issue], asset: &str, rule: Rule) {
    assert_eq!(issues.len(), 1, "{issues:#?}");
    assert_eq!((issues[0].asset.as_str(), issues[0].rule), (asset, rule));
    assert!(issues[0].to_string().contains(rule.name()));
    assert!(issues[0].to_string().starts_with(asset));
}

#[test]
fn good_fixture_passes() {
    let (c, i) = good();
    assert_eq!(run(&c, &i), vec![]);
}

#[test]
fn rule_format_version() {
    let (mut c, i) = good();
    c.format_version = 99;
    assert_only(&run(&c, &i), "<catalog>", Rule::FormatVersion);
}

#[test]
fn rule_duplicate_id() {
    let (mut c, i) = good();
    let dup = c.assets[0].clone();
    c.assets.push(dup);
    assert_only(&run(&c, &i), "prop.barrel", Rule::DuplicateId);
}

#[test]
fn rule_unknown_class() {
    let (mut c, i) = good();
    c.assets[0].class = AssetClass::Unknown;
    assert_only(&run(&c, &i), "prop.barrel", Rule::UnknownClass);
}

#[test]
fn rule_unknown_tag() {
    let (mut c, i) = good();
    c.assets[0].tags.function.push("spaceport".into());
    c.assets[0].tags.free.push("anything goes".into());
    assert_only(&run(&c, &i), "prop.barrel", Rule::UnknownTag);
}

#[test]
fn rule_licence() {
    let (mut c, i) = good();
    c.assets[0].licence = "  ".into();
    assert_only(&run(&c, &i), "prop.barrel", Rule::Licence);
}

#[test]
fn rule_provenance() {
    let (mut c, i) = good();
    c.assets[0].provenance = String::new();
    assert_only(&run(&c, &i), "prop.barrel", Rule::Provenance);
}

#[test]
fn rule_rotation() {
    let (mut c, i) = good();
    c.assets[0].rotations = vec![0, 45];
    assert_only(&run(&c, &i), "prop.barrel", Rule::Rotation);
    c.assets[0].rotations = vec![90, 90];
    assert_only(&run(&c, &i), "prop.barrel", Rule::Rotation);
}

#[test]
fn rule_class_fields() {
    let (mut c, i) = good();
    c.assets[1].ground = None;
    assert_only(&run(&c, &i), "ground.grass", Rule::ClassFields);
}

#[test]
fn rule_wall_kit() {
    let (mut c, i) = good();
    c.assets
        .retain(|a| a.wall.as_ref().is_none_or(|w| w.role != WallRole::Tee));
    assert_only(&run(&c, &i), "kit:stone", Rule::WallKit);
}

#[test]
fn rule_image_missing() {
    let (c, mut i) = good();
    i.insert("prop.barrel".into(), Err("no such file".into()));
    assert_only(&run(&c, &i), "prop.barrel", Rule::ImageMissing);
}

#[test]
fn rule_image_size() {
    let (c, mut i) = good();
    i.insert("prop.barrel".into(), Ok(Rgba::new(PPSQ + 1, PPSQ)));
    assert_only(&run(&c, &i), "prop.barrel", Rule::ImageSize);
}

#[test]
fn rule_alpha_opaque() {
    let (c, mut i) = good();
    i.insert(
        "prop.barrel".into(),
        Ok(Rgba::filled(PPSQ, PPSQ, [9, 9, 9, 255])),
    );
    assert_only(&run(&c, &i), "prop.barrel", Rule::AlphaOpaque);
}

#[test]
fn rule_alpha_fringe() {
    let (c, mut i) = good();
    let mut img = disc();
    // A faint glow in the corner, far from the opaque disc.
    for y in 0..4 {
        for x in 0..4 {
            img.set(x, y, [255, 255, 200, 40]);
        }
    }
    i.insert("prop.barrel".into(), Ok(img));
    assert_only(&run(&c, &i), "prop.barrel", Rule::AlphaFringe);
}

#[test]
fn antialiased_edges_are_not_fringe() {
    // `disc()` has a 1-px semi-transparent ring; `good_fixture_passes` covers
    // it, and this pins the ring as the reason the rule exists.
    let img = disc();
    assert!(img.data.as_chunks::<4>().0.iter().any(|p| p[3] == 128));
}

#[test]
fn rule_texture_alpha() {
    let (c, mut i) = good();
    let mut img = texture();
    img.set(3, 3, [0, 0, 0, 0]);
    i.insert("ground.grass".into(), Ok(img));
    assert_only(&run(&c, &i), "ground.grass", Rule::TextureAlpha);
}

#[test]
fn rule_tile_seam() {
    let (c, mut i) = good();
    let mut img = Rgba::new(PPSQ, PPSQ);
    for y in 0..PPSQ {
        for x in 0..PPSQ {
            // A smooth ramp: tiny interior steps, a big jump at the wrap.
            let v = (x * 12) as u8;
            img.set(x, y, [v, v, v, 255]);
        }
    }
    assert!(seam_ratio(&img) > 2.0);
    assert!(seam_ratio(&texture()) < 2.0);
    i.insert("ground.grass".into(), Ok(img));
    assert_only(&run(&c, &i), "ground.grass", Rule::TileSeam);
}

#[test]
fn check_dir_reports_missing_and_escaping_images() {
    let dir = std::env::temp_dir().join(format!("arda-tactical-check-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (mut c, _) = good();
    c.assets.truncate(1);
    c.assets[0].image = "../outside.png".into();
    std::fs::write(
        dir.join("catalog.json"),
        crate::catalog::to_json(&c).unwrap(),
    )
    .unwrap();
    let issues = crate::library::check_dir(&dir, &Thresholds::default()).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert_only(&issues, "prop.barrel", Rule::ImageMissing);
    assert!(issues[0].message.contains("inside the library"));
}

#[test]
fn overflowing_footprints_are_an_image_size_issue_not_a_panic() {
    let (mut c, i) = good();
    c.assets[0].footprint = Footprint {
        w: u32::MAX / 4,
        h: 1,
    };
    assert_only(&run(&c, &i), "prop.barrel", Rule::ImageSize);
}

#[test]
fn footprints_beyond_the_sprite_cap_are_an_image_size_issue() {
    let (mut c, mut i) = good();
    let side = crate::validate::MAX_FOOTPRINT_SQUARES + 1;
    c.assets[0].footprint = Footprint { w: side, h: 1 };
    // The image matches the footprint, so only the cap can object.
    let mut img = Rgba::new(side * PPSQ, PPSQ);
    for x in 0..side * PPSQ {
        img.set(x, 8, [120, 80, 40, 255]);
    }
    i.insert("prop.barrel".into(), Ok(img));
    let issues = run(&c, &i);
    assert_only(&issues, "prop.barrel", Rule::ImageSize);
    assert!(issues[0].to_string().contains("limit"), "{}", issues[0]);
}
