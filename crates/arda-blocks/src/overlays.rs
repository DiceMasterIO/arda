//! The overlay seam: ways, fields and town layers composed onto the refined
//! terrain by reservation precedence (logic/09 §reservations).
//!
//! Each layer is a full-size `TacticalLayout` of the same window plus its
//! `RulesSidecar` format 2 and an `owned` mask of the squares it reserves.
//! [`compose`] lets a layer claim an owned square only when the square's
//! current owner has lower precedence, and never lets it turn base water
//! into dry land: a layer over water must keep water or lay a deck (a road
//! over water is a bridge or ford; a building never stands on water,
//! logic/09 §reservations). A claimed square takes the layer's ground,
//! water, rules, placements, lights and walls; its natural scatter and
//! natural rules are dropped (Invariant 7: one owner per square).

use crate::BlocksError;
use arda_scene::{RulesCell, RulesSidecar};
use arda_tactical::layout::{EdgeAxis, TacticalLayout};
use arda_tactical::Library;

/// Who reserves a square, in rising precedence (logic/09 §reservations:
/// town 1 > ways 2 > water 3 > fields 4 > natural ground). Two soft
/// claims sit between: a road's verge yields to fields, and the open
/// ground of a town's footprint (crofts) yields to water and roads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Owner {
    /// Free natural ground and cliffs, from arda-refine.
    Natural,
    /// A road's verge that kept the natural ground, from arda-ways.
    Verge,
    /// Farmland, pasture, compounds and lanes, from arda-fields.
    Fields,
    /// Paddocks, orchards and gardens of a town's footprint, from arda-town.
    Croft,
    /// Rivers, lakes and sea, from arda-refine.
    Water,
    /// Roads, bridges and fords, from arda-ways.
    Ways,
    /// Streets, plots, buildings and walls, from arda-town.
    Town,
}

impl Owner {
    /// Short name for metadata.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Verge => "verge",
            Self::Fields => "fields",
            Self::Croft => "croft",
            Self::Water => "water",
            Self::Ways => "ways",
            Self::Town => "town",
        }
    }

    /// The precedence of this layer's soft claims.
    #[must_use]
    pub const fn soft(self) -> Self {
        match self {
            Self::Ways => Self::Verge,
            Self::Town => Self::Croft,
            other => other,
        }
    }
}

/// What an overlay provider sees: the window in world squares, the world
/// seed, the refined base layout and the library the result must resolve
/// against.
#[derive(Debug, Clone, Copy)]
pub struct OverlayCtx<'a> {
    /// First square column (world squares, 64 per cell).
    pub gsx0: i64,
    /// First square row.
    pub gsy0: i64,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// World seed.
    pub seed: u64,
    /// The refined layout of the window, before any overlay.
    pub base: &'a TacticalLayout,
    /// The asset library; layers degrade missing keys against it.
    pub library: &'a Library,
}

/// One overlay's contribution to a window.
#[derive(Debug, Clone)]
pub struct OverlayLayer {
    /// The layer's layout of the whole window (same size as the base).
    pub layout: TacticalLayout,
    /// Its rules, format 2, same size.
    pub rules: RulesSidecar,
    /// Row-major: the squares it reserves.
    pub owned: Vec<bool>,
    /// Row-major: owned squares claimed only at the layer's soft precedence
    /// ([`Owner::soft`]); empty for none.
    pub soft: Vec<bool>,
    /// Whether claimed squares take the layer's elevation (a layer built on
    /// the world's terrain) or keep the base's (a layer built on synthetic
    /// terrain, such as the demo samples).
    pub elevation: bool,
}

/// Supplies the ways, fields and town layers of a window. Every method
/// must be a pure function of global coordinates and the seed, so that
/// neighbouring windows agree (goal 46). A provider without a layer
/// answers `Ok(None)`.
pub trait Overlays: Send + Sync + std::fmt::Debug {
    /// Roads, bridges and fords (arda-ways).
    ///
    /// # Errors
    /// [`BlocksError::Overlay`].
    fn ways(&self, _ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        Ok(None)
    }

    /// Fields, compounds and lanes (arda-fields).
    ///
    /// # Errors
    /// [`BlocksError::Overlay`].
    fn fields(&self, _ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        Ok(None)
    }

    /// Streets, plots and buildings (arda-town).
    ///
    /// # Errors
    /// [`BlocksError::Overlay`].
    fn town(&self, _ctx: &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError> {
        Ok(None)
    }
}

/// The two squares an edge separates, as signed local coordinates.
const fn sides(axis: EdgeAxis, x: u32, y: u32) -> [(i64, i64); 2] {
    let (x, y) = (x as i64, y as i64);
    match axis {
        EdgeAxis::Horizontal => [(x, y), (x, y - 1)],
        EdgeAxis::Vertical => [(x, y), (x - 1, y)],
    }
}

/// Composes one layer onto `layout`/`rules`, updating `owners`. Returns
/// the number of squares the layer claimed.
///
/// # Errors
/// [`BlocksError::Overlay`] when the layer's size differs from the base.
pub fn compose(
    layout: &mut TacticalLayout,
    rules: &mut RulesSidecar,
    owners: &mut [Owner],
    layer: &OverlayLayer,
    owner: Owner,
) -> Result<usize, BlocksError> {
    let (w, h) = (layout.width, layout.height);
    let n = w as usize * h as usize;
    let fits = layer.layout.width == w
        && layer.layout.height == h
        && layer.layout.squares.len() == n
        && layer.owned.len() == n
        && owners.len() == n;
    if !fits {
        return Err(BlocksError::Overlay {
            layer: owner.name(),
            message: format!("layer is not the {w}x{h} window"),
        });
    }
    layer.rules.check(w, h)?;
    let rank = |i: usize| {
        if layer.soft.get(i).copied().unwrap_or(false) {
            owner.soft()
        } else {
            owner
        }
    };
    let claimed: Vec<bool> = (0..n)
        .map(|i| {
            let (sq, cell) = (&layer.layout.squares[i], &layer.rules.squares[i]);
            let keeps_water = sq.water_depth_ft > 0 || cell.deck == Some(true);
            let base_wet = layout.squares[i].water_depth_ft > 0;
            layer.owned[i] && owners[i] < rank(i) && (keeps_water || !base_wet)
        })
        .collect();
    let at = |x: i64, y: i64| -> Option<usize> {
        let (xu, yu) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
        (xu < w && yu < h).then(|| yu as usize * w as usize + xu as usize)
    };
    // The square a point in squares falls in, clamped into the window.
    let anchor = |px: f32, py: f32| -> usize {
        #[allow(clippy::cast_possible_truncation)]
        let (x, y) = (px.floor() as i64, py.floor() as i64);
        let (x, y) = (x.clamp(0, i64::from(w) - 1), y.clamp(0, i64::from(h) - 1));
        at(x, y).unwrap_or(0)
    };
    let mut count = 0;
    for i in 0..n {
        if !claimed[i] {
            continue;
        }
        count += 1;
        let (base, up) = (&mut layout.squares[i], &layer.layout.squares[i]);
        let deck = layer.rules.squares[i].deck == Some(true);
        base.ground.clone_from(&up.ground);
        if up.water_depth_ft > 0 || !deck {
            base.water_depth_ft = up.water_depth_ft;
        }
        if layer.elevation {
            base.elevation_ft = up.elevation_ft;
        }
        owners[i] = rank(i);
        let mut cell = RulesCell::default();
        cell.merge_from(&layer.rules.squares[i]);
        rules.squares[i] = cell;
    }
    let touches = |axis, x, y| {
        sides(axis, x, y)
            .iter()
            .any(|&(sx, sy)| at(sx, sy).is_some_and(|k| claimed[k]))
    };
    for wall in &layer.layout.walls {
        if touches(wall.axis, wall.x, wall.y) {
            layout
                .walls
                .retain(|b| (b.x, b.y, b.axis) != (wall.x, wall.y, wall.axis));
            layout.walls.push(wall.clone());
        }
    }
    for e in &layer.rules.edges {
        if touches(e.axis, e.x, e.y) {
            rules
                .edges
                .retain(|b| (b.x, b.y, b.axis) != (e.x, e.y, e.axis));
            rules.edges.push(e.clone());
        }
    }
    layout.placements.retain(|p| !claimed[anchor(p.x, p.y)]);
    layout.placements.extend(
        layer
            .layout
            .placements
            .iter()
            .filter(|p| claimed[anchor(p.x, p.y)])
            .cloned(),
    );
    layout.lights.retain(|l| !claimed[anchor(l.x, l.y)]);
    layout.lights.extend(
        layer
            .layout
            .lights
            .iter()
            .filter(|l| claimed[anchor(l.x, l.y)])
            .copied(),
    );
    Ok(count)
}

/// One provider method.
type LayerFn = fn(&dyn Overlays, &OverlayCtx<'_>) -> Result<Option<OverlayLayer>, BlocksError>;

/// Applies the providers' layers in the fixed order ways, fields, town.
/// Returns the names of the layers that claimed at least one square.
///
/// # Errors
/// A provider's error, or a layer that does not fit the window.
pub fn compose_all(
    ctx: &OverlayCtx<'_>,
    overlays: &dyn Overlays,
    layout: &mut TacticalLayout,
    rules: &mut RulesSidecar,
    owners: &mut [Owner],
) -> Result<Vec<&'static str>, BlocksError> {
    let mut applied = Vec::new();
    let steps: [(Owner, LayerFn); 3] = [
        (Owner::Ways, |o, c| o.ways(c)),
        (Owner::Fields, |o, c| o.fields(c)),
        (Owner::Town, |o, c| o.town(c)),
    ];
    for (owner, get) in steps {
        if let Some(layer) = get(overlays, ctx)? {
            if compose(layout, rules, owners, &layer, owner)? > 0 {
                applied.push(owner.name());
            }
        }
    }
    rules.edges.sort_by_key(|e| (e.axis, e.y, e.x));
    layout.walls.sort_by_key(|w| (w.axis, w.y, w.x));
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(ground: &str, owned: Vec<bool>, soft: Vec<bool>) -> OverlayLayer {
        let mut layout = TacticalLayout::new("t", 3, 1, ground);
        layout
            .squares
            .iter_mut()
            .for_each(|s| ground.clone_into(&mut s.ground));
        OverlayLayer {
            layout,
            rules: RulesSidecar::empty(3, 1),
            owned,
            soft,
            elevation: false,
        }
    }

    #[test]
    fn soft_claims_yield_to_the_layers_they_rank_below() {
        let mut base = TacticalLayout::new("t", 3, 1, "forest_floor");
        let mut rules = RulesSidecar::empty(3, 1);
        let mut owners = vec![Owner::Natural; 3];
        // A road on square 0 and its verge on square 1.
        let ways = layer("dirt", vec![true, true, false], vec![false, true, false]);
        compose(&mut base, &mut rules, &mut owners, &ways, Owner::Ways).unwrap();
        assert_eq!(owners, [Owner::Ways, Owner::Verge, Owner::Natural]);
        // Fields take the verge but not the road.
        let fields = layer("pasture", vec![true; 3], Vec::new());
        compose(&mut base, &mut rules, &mut owners, &fields, Owner::Fields).unwrap();
        assert_eq!(owners, [Owner::Ways, Owner::Fields, Owner::Fields]);
        // Crofts take fields but give way to the road; a building takes all.
        let town = layer("grass", vec![true; 3], vec![true, true, false]);
        compose(&mut base, &mut rules, &mut owners, &town, Owner::Town).unwrap();
        assert_eq!(owners, [Owner::Ways, Owner::Croft, Owner::Town]);
        let grounds: Vec<&str> = base.squares.iter().map(|s| s.ground.as_str()).collect();
        assert_eq!(grounds, ["dirt", "grass", "grass"]);
    }
}
