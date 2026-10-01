use super::*;

/// A temperate eco on dry, level ground far from water.
fn base(temp: f64) -> Eco {
    Eco {
        temp,
        growth: smoothstep(-1.5, 3.5, temp),
        conifer: smoothstep(9.5, 3.5, temp),
        wet: 0.3,
        water_d: 40.0,
        fd: 0.8,
        moist: 0.5,
        slope: 2.0,
        ..Eco::default()
    }
}

/// Share of draws over a fine grid whose species satisfies `f`.
fn share(e: &Eco, large: bool, f: impl Fn(&str) -> bool) -> f64 {
    let n = 60;
    let mut hit = 0;
    for i in 0..n {
        for j in 0..n {
            let (r, q) = ((f64::from(i) + 0.5) / 60.0, (f64::from(j) + 0.5) / 60.0);
            hit += u32::from(f(tree(e, large, r, q).0));
        }
    }
    f64::from(hit) / f64::from(n * n)
}

fn conifer(id: &str) -> bool {
    matches!(id, "veg.tree_pine" | "veg.tree_spruce")
}

#[test]
fn conifers_climb_and_broadleaves_stay_low() {
    let warm = share(&base(11.0), true, conifer);
    let cool = share(&base(6.0), true, conifer);
    let cold = share(&base(3.5), true, conifer);
    assert!(warm < 0.15, "warm lowland conifers {warm}");
    assert!(cool > warm + 0.2, "cool {cool} vs warm {warm}");
    assert!(cold > 0.6, "cold upland conifers {cold}");
    let oak = |id: &str| matches!(id, "veg.tree_oak" | "veg.tree_elm");
    assert!(share(&base(11.0), true, oak) > 0.5);
    assert!(share(&base(3.5), true, oak) < 0.1);
}

#[test]
fn shaded_slopes_are_colder_and_more_coniferous() {
    let src = crate::synthetic::world(1);
    let ctx = Ctx::gather(&src, crate::synthetic::FOREST_CELL).unwrap();
    let slope = |north: f64| Phys {
        slope_deg: 25.0,
        north,
        ..Phys::default()
    };
    let (u, v) = (2.0 * 64.0 + 20.0, 2.0 * 64.0 + 20.0);
    let shade = eco(&ctx, &slope(1.0), Shape::default(), u, v);
    let sun = eco(&ctx, &slope(-1.0), Shape::default(), u, v);
    assert!(
        shade.temp < sun.temp - 2.5,
        "{} vs {}",
        shade.temp,
        sun.temp
    );
    assert!(shade.conifer > sun.conifer);
    assert!(share(&shade, true, conifer) > share(&sun, true, conifer));
}

#[test]
fn willows_and_alders_line_the_water() {
    let riparian = |id: &str| matches!(id, "veg.tree_willow" | "veg.tree_alder");
    let mut bank = base(10.0);
    bank.water_d = 1.0;
    bank.wet = 0.9;
    let far = base(10.0);
    assert!(share(&bank, true, riparian) > 0.6);
    assert!(share(&far, true, riparian) < 0.02);
    // Cool streams get alder rather than willow.
    let mut cold = bank;
    cold.temp = 5.0;
    assert!(share(&cold, true, |id| id == "veg.tree_willow") < 1e-9);
}

#[test]
fn the_tree_line_stunts_and_kills() {
    let stunted = |id: &str| matches!(id, "veg.tree_stunted" | "veg.tree_dead");
    let low = share(&base(9.0), true, stunted);
    let line = share(&base(0.5), true, stunted);
    assert!(low < 0.03, "valley {low}");
    assert!(line > 0.4, "tree line {line}");
}

#[test]
fn low_plants_follow_shade_wetness_and_sun() {
    let mut forest = base(9.0);
    forest.moist = 0.8;
    assert_eq!(low_plant(&forest, 1.0, 0.5).0, "veg.fern");
    let mut meadow = base(12.0);
    meadow.grass = 1.0;
    meadow.sun = 1.0;
    meadow.wet = 0.1;
    let pick = |e: &Eco, id: &str| {
        (0..20)
            .filter(|&i| low_plant(e, 0.0, (f64::from(i) + 0.5) / 20.0).0 == id)
            .count()
    };
    assert!(pick(&meadow, "veg.flower_patch") >= 8);
    let mut fen = meadow;
    fen.wet = 1.0;
    fen.sun = 0.0;
    assert!(pick(&fen, "veg.tall_grass") >= 8);
    let mut fell = base(3.0);
    fell.scrub = 1.0;
    fell.shape.hollow = -0.8;
    fell.wet = 0.1;
    assert!(pick(&fell, "veg.heather") >= 8);
}
