use super::*;
use crate::layout::Square;

fn layout(origin: [i64; 2], ground: &str) -> TacticalLayout {
    TacticalLayout {
        name: "tint".into(),
        width: 8,
        height: 8,
        squares: vec![
            Square {
                ground: ground.into(),
                elevation_ft: 0,
                water_depth_ft: 0,
                dryness: 0,
            };
            64
        ],
        walls: Vec::new(),
        placements: Vec::new(),
        lights: Vec::new(),
        origin: Some(origin),
    }
}

/// A lattice every 4 squares around world squares 0..32, colour by position.
fn tint() -> WorldTint {
    let size = (10, 10);
    let ground = (0..100)
        .map(|i| {
            [
                u8::try_from(40 + i).unwrap(),
                120,
                u8::try_from(200 - i).unwrap(),
            ]
        })
        .collect();
    WorldTint {
        origin: [-4, -4],
        step: 4,
        size,
        ground,
        water: vec![[30, 90, 140]; 100],
        strength_q12: DEFAULT_STRENGTH_Q12,
    }
}

fn paint(origin: [i64; 2], ground: &str) -> Rgba {
    let mut img = Rgba::filled(64, 64, [120, 120, 60, 255]);
    let water = vec![0_u8; 64 * 64];
    let textures = TextureSet::default();
    apply(
        &mut img,
        &layout(origin, ground),
        (&textures, &water),
        8,
        &tint(),
        [0, 0, 64, 64],
    )
    .unwrap();
    img
}

#[test]
fn shared_world_pixels_match_whatever_the_render_window() {
    // Two 8 × 8-square renders offset by 3 squares share a 5 × 8 strip.
    let a = paint([4, 4], "grass");
    let b = paint([7, 4], "grass");
    for y in 0..64 {
        for x in 0..40 {
            assert_eq!(a.get(x + 24, y), b.get(x, y), "{x},{y}");
        }
    }
}

#[test]
fn laid_surfaces_take_less_of_the_tint() {
    let grass = paint([4, 4], "grass");
    let street = paint([4, 4], "cobbles");
    let base = [120_u8, 120, 60, 255];
    let moved = |img: &Rgba| {
        let p = img.get(30, 30);
        (0..3)
            .map(|c| i32::from(p[c]).abs_diff(i32::from(base[c])))
            .sum::<u32>()
    };
    assert!(
        moved(&grass) > 2 * moved(&street),
        "{} {}",
        moved(&grass),
        moved(&street)
    );
}

#[test]
fn malformed_or_misaligned_lattices_are_refused() {
    let mut img = Rgba::filled(64, 64, [1, 2, 3, 255]);
    let water = vec![0_u8; 64 * 64];
    let textures = TextureSet::default();
    let mut t = tint();
    t.origin = [-3, -4];
    let l = layout([4, 4], "grass");
    let run =
        |t: &WorldTint, img: &mut Rgba| apply(img, &l, (&textures, &water), 8, t, [0, 0, 64, 64]);
    assert!(run(&t, &mut img).is_err(), "off the lattice");
    let mut small = tint();
    small.size = (3, 3);
    small.ground.truncate(9);
    small.water.truncate(9);
    assert!(run(&small, &mut img).is_err(), "too small");
    assert_eq!(img.get(0, 0), [1, 2, 3, 255], "refusals leave the canvas");
}
