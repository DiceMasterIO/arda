//! Processing one raw file into a catalogue-ready image.

use crate::cleanup::{defringe, kill_haze, remove_specks};
use crate::fit::{fit_canvas, fit_cutout, fit_fill, fit_vegetation};
use crate::holes::HolePolicy;
use crate::image_io::RawImage;
use crate::manifest::{HoleMode, ShadowMode};
use crate::naming::{role_name, Target};
use crate::shadow;
use crate::texture;
use crate::wall::{self, Orientation};
use arda_tactical::catalog::{Asset, Layer, WallRole};
use arda_tactical::Rgba;

/// A processed asset, before grading and validation.
#[derive(Debug, Clone)]
pub struct Processed {
    /// The catalogue record.
    pub asset: Asset,
    /// What it was made from.
    pub target: Target,
    /// Raw file name.
    pub file: String,
    /// The image, `footprint × ppsq`.
    pub img: Rgba,
    /// What was done.
    pub fixes: Vec<String>,
    /// What needs a look.
    pub flags: Vec<String>,
}

/// Runs the class's pipeline on one raw image: `ppsq`, the shadow mode and
/// the manifest's `holes` key (`None`: the class default).
#[must_use]
pub fn process(
    raw: &RawImage,
    asset: Asset,
    target: Target,
    file: String,
    (ppsq, mode, holes): (u32, ShadowMode, Option<HoleMode>),
) -> Processed {
    let mut p = Processed {
        img: Rgba::new(1, 1),
        asset,
        target,
        file,
        fixes: Vec::new(),
        flags: Vec::new(),
    };
    if let Target::Texture { .. } = p.target {
        let structured = crate::meta::is_structured(&p.asset);
        let t = texture::prepare(&raw.rgba, p.asset.footprint, ppsq, structured);
        p.img = t.img;
        p.fixes.extend(t.fixes);
        p.flags.extend(t.flags);
        return p;
    }
    let holes = HolePolicy::for_asset(&p.target, holes);
    let mut img = matte(raw, (mode, holes), &mut p.fixes, &mut p.flags);
    let specks = remove_specks(&mut img);
    if specks > 0 {
        p.fixes.push(format!("removed {specks} px of stray specks"));
    }
    let haze = kill_haze(&mut img);
    if haze > 0 {
        p.fixes
            .push(format!("cleared {haze} px of haze or halo beyond 2 px"));
    }
    if raw.has_alpha {
        let n = defringe(&mut img);
        if n > 0 {
            p.fixes
                .push(format!("defringed {n} edge px toward the object colour"));
        }
    }
    let fitted = match &p.target {
        Target::Wall { .. } => fit_canvas(&img, ppsq),
        _ if p.asset.layer == Layer::Floor => fit_fill(&img, p.asset.footprint, ppsq),
        Target::Vegetation { .. } => fit_vegetation(&img, p.asset.footprint, ppsq),
        _ => fit_cutout(&img, p.asset.footprint, ppsq, true),
    };
    p.img = fitted.img;
    p.fixes.extend(fitted.fixes);
    p.flags.extend(fitted.flags);
    if let Target::Wall { role, .. } = p.target {
        orient(&mut p, role);
    }
    let late = kill_haze(&mut p.img);
    if late > 0 {
        p.fixes
            .push(format!("cleared {late} px of resampling haze"));
    }
    if let Some(f) = shadow::residual_flag(&p.img) {
        p.flags.push(f);
    }
    if p.img.data.as_chunks::<4>().0.iter().all(|px| px[3] == 255) {
        p.flags
            .push("no transparent pixels: the background was not removed".into());
    }
    p
}

/// Background removal (opaque sources) or shadow stripping (sources with
/// alpha).
fn matte(
    raw: &RawImage,
    (mode, holes): (ShadowMode, HolePolicy),
    fixes: &mut Vec<String>,
    flags: &mut Vec<String>,
) -> Rgba {
    if !raw.has_alpha {
        let cut = crate::matte::cut_out(&raw.rgba, mode, holes);
        fixes.extend(cut.fixes);
        flags.extend(cut.flags);
        return cut.img;
    }
    fixes.push("kept the source alpha".into());
    let mut img = raw.rgba.clone();
    let found = shadow::grow_in_alpha(&img);
    let mut removed = vec![false; found.mask.len()];
    shadow::apply(&found, mode, &mut removed, fixes, flags);
    for (p, r) in img.data.as_chunks_mut::<4>().0.iter_mut().zip(&removed) {
        if *r {
            *p = [0; 4];
        }
    }
    img
}

/// Turns a wall piece to its canonical arms, or flags it; checks that edge
/// pieces reach both ends of the edge.
fn orient(p: &mut Processed, role: WallRole) {
    match wall::check(&p.img, role) {
        Orientation::Canonical => {}
        Orientation::Turn(k) => {
            p.img = p.img.rotated(k);
            p.fixes.push(format!(
                "turned {}° clockwise to the canonical {} arms ({})",
                u32::from(k) * 90,
                role_name(role),
                wall::arm_names(wall::canonical(role))
            ));
        }
        Orientation::Mismatch(m) => p.flags.push(m),
    }
    if role.is_edge() && !wall::reaches_ends(&p.img) {
        p.flags.push(
            "an edge piece should run from the west edge to the east edge of its square".into(),
        );
    }
}
