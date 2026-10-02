//! Hierarchical subdivision of one land block into fields.
//!
//! A region is split across its long axis (or along the contour on a
//! slope, or lengthwise into long narrow closes) at a jittered position,
//! with a slight kink in old enclosure, until it reaches the field size
//! its ground asks for ([`Context::target_sq`]). Where a road crosses a
//! region that is still to be split, the road itself is the cut, so fields
//! line the roads; where land use changes within a region, the cut that
//! best separates it wins, so woods and meadows keep field-edge outlines.
//! Every cut ends on its parent's boundary: junctions are T-shaped.
//! A cut must meet that boundary at [`MIN_CORNER_DEG`] or more, so no
//! field gets a sharp wedge corner; where every cut would, the region
//! stays whole and the sliver remains part of its neighbour. Only a road
//! may force a sharper corner, and only where the road itself runs.

use super::context::{ancient_at, leaf_use, separation, Context, Ground};
use super::cut::Cut;
use super::guide::{fit, stretch, Guide};
use super::lattice::{outline, BlockId};
use super::poly::{
    bbox, clip_region, closest_on_outline, closest_on_segment, dot, extent, major_axis, moments,
    perp, region_contains, rotate, sub, unit, Moments, P,
};
use super::Site;
use crate::fields::FieldKind;
use crate::geom::{h3, s11, u01};

/// Smallest field, squares (0.16 ha).
pub const MIN_AREA: f64 = 640.0;
/// Narrowest field across, squares (19 m).
const MIN_WIDTH: f64 = 12.0;
/// Longest field, squares (280 m).
pub const MAX_LEN: f64 = 180.0;
/// Regions of mixed land use split down to this size, squares (1.2 ha).
const MIXED_STOP: f64 = 4_800.0;
/// Least share of its principal-axis box a field fills (a right triangle
/// fills a half, a rectangle all of it).
const MIN_FILL: f64 = 0.64;
/// Regions larger than this take their own grain, squares (20 ha).
const GRAIN_AREA: f64 = 81_920.0;
/// Deepest split.
const MAX_DEPTH: u32 = 24;
/// Least angle, degrees, at which a cut may meet its region's boundary:
/// sharper corners make wedge-shaped fields. A little above the 35° the
/// shape tests hold every field to, for the warp and the rasterisation.
pub const MIN_CORNER_DEG: f64 = 38.0;
/// How far outside a region's clipped outline a meeting with a cut above
/// it still counts as on its boundary, squares (a road's stand-in line
/// strays from the road by a few squares).
const NEAR_OUTLINE: f64 = 6.0;

/// A node of a block's tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// An inner node: its cut and the children on its right (`0`) and
    /// left-normal (`1`) sides.
    Split {
        /// The cut.
        cut: Cut,
        /// Child node indices.
        kids: [u32; 2],
    },
    /// A field: index into the block's sites.
    Leaf(u32),
}

/// One block's field tree.
#[derive(Debug, Clone)]
pub struct Tree {
    /// The block.
    pub id: BlockId,
    /// Its outline (warped squares).
    pub outline: Vec<P>,
    /// Nodes; the root is node 0.
    pub nodes: Vec<Node>,
}

/// A candidate cut with the straight line standing in for it when
/// clipping the children's geometry.
struct Candidate {
    cut: Cut,
    /// `Some((c, d1, d2))` for a kinked cut.
    kink: Option<(P, P, P)>,
    /// A point on and the direction of the stand-in line.
    line: (P, P),
    /// Whether the children must be field-shaped (not wedges).
    strict: bool,
    /// A road: it may meet the boundary at any angle (its end rays may not).
    road: bool,
}

impl Candidate {
    fn straight(c: P, d: P) -> Self {
        Self {
            cut: Cut::line(c, d),
            kink: None,
            line: (c, d),
            strict: true,
            road: false,
        }
    }

    /// The children's geometry: right side first, then left.
    fn children(&self, r: &[Vec<P>]) -> [Vec<Vec<P>>; 2] {
        match self.kink {
            Some((c, d1, d2)) => {
                let m = unit([d1[0] + d2[0], d1[1] + d2[1]]);
                let (back, ahead) = (clip_region(r, [-m[0], -m[1]], c), clip_region(r, m, c));
                let (n1, n2) = (perp(d1), perp(d2));
                let neg = |n: P| [-n[0], -n[1]];
                let mut right = clip_region(&back, neg(n1), c);
                right.extend(clip_region(&ahead, neg(n2), c));
                let mut left = clip_region(&back, n1, c);
                left.extend(clip_region(&ahead, n2, c));
                [right, left]
            }
            None => {
                let (c, d) = self.line;
                let n = perp(d);
                [clip_region(r, [-n[0], -n[1]], c), clip_region(r, n, c)]
            }
        }
    }
}

struct Builder<'a, 'g> {
    seed: u64,
    g: &'a Ground<'g>,
    guides: &'a [Guide],
    id: BlockId,
    origin: P,
    /// The block's outline, closed.
    rim: Vec<P>,
    nodes: Vec<Node>,
    sites: Vec<Site>,
    /// The cuts above the node being split, root first.
    above: Vec<Cut>,
}

/// Builds the field tree of block `id` and its fields.
#[must_use]
pub fn build(seed: u64, g: &Ground<'_>, guides: &[Guide], id: BlockId) -> (Tree, Vec<Site>) {
    let poly = outline(seed, id);
    let mut b = Builder {
        seed,
        g,
        guides,
        id,
        origin: poly[0],
        rim: poly.iter().chain(poly.first()).copied().collect(),
        nodes: Vec::new(),
        sites: Vec::new(),
        above: Vec::new(),
    };
    b.node(std::slice::from_ref(&poly), 1, 0);
    (
        Tree {
            id,
            outline: poly,
            nodes: b.nodes,
        },
        b.sites,
    )
}

/// A hash-drawn value in `[0, 1)` for decision `k` of a node.
fn draw(h: u64, k: u64) -> f64 {
    u01(h ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

impl Builder<'_, '_> {
    fn node(&mut self, region: &[Vec<P>], path: u32, depth: u32) -> u32 {
        let me = u32::try_from(self.nodes.len()).unwrap_or(u32::MAX);
        self.nodes.push(Node::Leaf(0));
        let mo = moments(region, self.origin);
        let mut ctx = Context::new(self.g, region, mo.centroid, mo.area);
        ctx.ancient = ancient_at(self.seed, mo.centroid, self.g.style);
        let h = h3(self.seed, 0x5D17, self.id.0, self.id.1, i64::from(path));
        let e1 = major_axis(mo.cov);
        let (lo, hi) = extent(region, e1);
        let l1 = hi - lo;
        let (lo2, hi2) = extent(region, perp(e1));
        let l2 = hi2 - lo2;
        // Field sizes vary about the target: 0.4× to 2.2×.
        let jitter = (0.4_f64.ln() + draw(h, 1) * (2.2_f64 / 0.4).ln()).exp();
        let target = ctx.target_sq(self.g.style) * jitter;
        let fits =
            mo.area <= target && l1 <= MAX_LEN && (ctx.purity >= 0.8 || mo.area <= MIXED_STOP);
        let small = mo.area < 2.4 * MIN_AREA;
        if !(fits || small || depth >= MAX_DEPTH) {
            for cand in self.candidates(region, &mo, &ctx, h, (e1, l1, l2), target) {
                let kids = cand.children(region);
                if !self.valid(region, &kids, &cand) {
                    continue;
                }
                let [right, left] = kids;
                self.above.push(cand.cut.clone());
                let a = self.node(&right, path * 2, depth + 1);
                let b = self.node(&left, path * 2 + 1, depth + 1);
                self.above.pop();
                self.nodes[me as usize] = Node::Split {
                    cut: cand.cut,
                    kids: [a, b],
                };
                return me;
            }
        }
        let leaf = self.leaf(&mo, &ctx, path, h, (e1, l2));
        self.nodes[me as usize] = Node::Leaf(leaf);
        me
    }

    /// Both children are large and wide enough, and no corner the cut
    /// makes is a wedge (unless the cut is a road).
    fn valid(&self, region: &[Vec<P>], kids: &[Vec<Vec<P>>; 2], cand: &Candidate) -> bool {
        let n = perp(cand.line.1);
        let sized = kids.iter().all(|k| {
            let (lo, hi) = extent(k, n);
            let m = moments(k, self.origin);
            m.area >= MIN_AREA && hi - lo >= MIN_WIDTH && (!cand.strict || fill(k, &m) >= MIN_FILL)
        });
        if !sized {
            return false;
        }
        // A road meets the boundary as it must; where it ends inside the
        // region the cut runs on as a straight ray, which must not.
        let pts = cand.cut.points();
        let angle = if cand.road {
            let n = pts.len();
            self.corner_deg(region, &pts[..2], false)
                .min(self.corner_deg(region, &pts[n - 2..], false))
        } else {
            self.corner_deg(region, pts, true)
        };
        angle >= MIN_CORNER_DEG
    }

    /// The sharpest corner, degrees (0°–90°), that the polyline `cut`
    /// makes where it meets the boundary of `region`: its outline, and
    /// the block outline and cuts above it as they really run (a road's
    /// region is clipped along a straight stand-in, so the outline alone
    /// can miss where a cut meets the road, its end rays or the block
    /// edge beyond the stand-in). With `slivers`, a boundary
    /// edge that runs within [`MIN_WIDTH`] of the cut at a shallow angle
    /// without crossing it counts too: the cut would cut off a sliver
    /// whose sharp tip another edge blunts. 90° when it meets none.
    fn corner_deg(&self, region: &[Vec<P>], cut: &[P], slivers: bool) -> f64 {
        let mut least = 90.0_f64;
        let inside = |x: P| region_contains(region, x);
        for piece in region {
            let n = piece.len();
            for i in 0..n {
                let (a, b) = (piece[i], piece[(i + 1) % n]);
                let nrm = perp(unit(sub(b, a)));
                // Inside on both sides: an edge between two pieces.
                let inner = |x: P| {
                    let probe = |k: f64| [x[0] + nrm[0] * k, x[1] + nrm[1] * k];
                    inside(probe(0.05)) && inside(probe(-0.05))
                };
                for s in cut.windows(2) {
                    let Some((x, deg)) = crossing(s[0], s[1], a, b) else {
                        continue;
                    };
                    if !inner(x) {
                        least = least.min(deg);
                    }
                }
                let e = sub(b, a);
                let mid = [a[0] + e[0] * 0.5, a[1] + e[1] * 0.5];
                if slivers && dot(e, e) >= 4.0 && !inner(mid) {
                    let (gap, dir) = nearest_on(cut, mid);
                    let deg = (dot(unit(e), dir).abs().min(1.0)).acos().to_degrees();
                    let close = nearest_on(cut, a).0.min(nearest_on(cut, b).0);
                    if deg < MIN_CORNER_DEG && close.min(gap) < MIN_WIDTH {
                        least = least.min(deg);
                    }
                }
            }
        }
        let near = |x: P| {
            inside(x)
                || region
                    .iter()
                    .any(|piece| closest_on_outline(piece, x).0 <= NEAR_OUTLINE)
        };
        let rim = std::iter::once(self.rim.as_slice());
        for up in rim.chain(self.above.iter().map(Cut::points)) {
            for e in up.windows(2) {
                for s in cut.windows(2) {
                    if let Some((x, deg)) = crossing(s[0], s[1], e[0], e[1]) {
                        if deg < least && near(x) {
                            least = deg;
                        }
                    }
                }
            }
        }
        least
    }

    fn leaf(&mut self, mo: &Moments, ctx: &Context, path: u32, h: u64, axes: (P, f64)) -> u32 {
        let (e1, l2) = axes;
        let key = (self.id.0, self.id.1, path);
        let hkey = Site::hash_key(key);
        let (kind, crop) = leaf_use(ctx, mo.centroid, self.g, self.seed, hkey);
        // Strips run down the slope where there is one (so the furrows
        // drain), else along the furlong, sometimes across it.
        let along = if kind != FieldKind::Strips {
            e1
        } else if ctx.slope > 0.04 && draw(h, 10) < 0.7 {
            ctx.grad
        } else if l2 >= 90.0 && draw(h, 11) < 0.3 {
            perp(e1)
        } else {
            e1
        };
        self.sites.push(Site {
            key,
            p: mo.centroid,
            class: ctx.class,
            along,
            slope: ctx.slope,
            kind,
            crop,
            area: mo.area,
        });
        u32::try_from(self.sites.len() - 1).unwrap_or(u32::MAX)
    }

    /// Cuts to try, best first.
    fn candidates(
        &self,
        region: &[Vec<P>],
        mo: &Moments,
        ctx: &Context,
        h: u64,
        (e1, l1, l2): (P, f64, f64),
        target: f64,
    ) -> Vec<Candidate> {
        let mut out = Vec::new();
        if mo.area > 1.5 * target && l1 > 100.0 {
            if let Some(g) = self.guide(region, mo, l1.min(l2)) {
                out.push(g);
            }
        }
        let (across, lengthwise) = frontage_axes(region, e1);
        let styled = self.styled(region, mo, ctx, h, (e1, l1, l2));
        if let (true, Some(first)) = (ctx.purity < 0.8 && ctx.samples.len() >= 8, styled.first()) {
            // A straight cut that separates the land use clearly better
            // than the style's own.
            let n = ctx.samples.len();
            let (base, _) = separation(&ctx.samples, |p| first.cut.side(p));
            let mut best: Option<(usize, Candidate)> = None;
            for d in [across, lengthwise] {
                for t in [0.3, 0.5, 0.7] {
                    let c = at_share(region, mo.centroid, d, t);
                    let cand = Candidate::straight(c, d);
                    let (score, sides) = separation(&ctx.samples, |p| cand.cut.side(p));
                    if sides[0].min(sides[1]) * 6 < n || score < base + (n / 12).max(2) {
                        continue;
                    }
                    if best.as_ref().is_none_or(|(s, _)| score > *s) {
                        best = Some((score, cand));
                    }
                }
            }
            out.extend(best.map(|(_, c)| c));
        }
        out.extend(styled);
        // Last resorts, square to the frontage and then along it; if both
        // would leave a wedge, the region stays one field.
        for d in [across, lengthwise] {
            let mut last = Candidate::straight(at_share(region, mo.centroid, d, 0.5), d);
            last.strict = false;
            out.push(last);
        }
        out
    }

    /// The cut the enclosure style draws.
    fn styled(
        &self,
        region: &[Vec<P>],
        mo: &Moments,
        ctx: &Context,
        h: u64,
        (e1, l1, l2): (P, f64, f64),
    ) -> Vec<Candidate> {
        let planned = ctx.ancient < 0.5;
        let (across, lengthwise) = frontage_axes(region, e1);
        let mut d = across;
        let contour = perp(ctx.grad);
        let (ca, cl) = (dot(across, contour).abs(), dot(lengthwise, contour).abs());
        // Boundaries follow the contours on a slope, where the region's own
        // sides allow (cos 35° ≈ 0.82).
        if ctx.slope > 0.035 && ca.max(cl) > 0.82 && draw(h, 2) < 0.8 {
            let best = if cl > ca { lengthwise } else { across };
            let (lo, hi) = extent(region, perp(best));
            if hi - lo >= 3.0 * MIN_WIDTH {
                d = best;
            }
        } else {
            // Old enclosure near the houses: long narrow closes.
            let near = ctx.settle_m < 700.0 && !planned;
            let p_long = if near { 0.3 } else { 0.12 };
            if l2 > 3.0 * MIN_WIDTH && l1 < 2.2 * l2 && draw(h, 3) < p_long {
                d = lengthwise;
            }
        }
        // Large regions (a block, its halves) take their own grain, which
        // their fields then follow; fields wobble about it a little.
        let wobble = match (mo.area > GRAIN_AREA, planned) {
            (true, false) => 0.6,
            (true, true) => 0.2,
            (false, false) => 0.18,
            (false, true) => 0.035,
        };
        let mut turn = wobble * s11(h ^ 0x4C);
        if planned {
            // Planned enclosure follows one surveyed bearing across a
            // parish (a 6 km field), whatever the older lanes do.
            let n = super::lattice::vnoise(self.seed, 0xBEA1, mo.centroid, 4_000.0);
            let bearing = std::f64::consts::FRAC_PI_4 * (1.6 * n).clamp(-1.0, 1.0);
            let quarter = std::f64::consts::FRAC_PI_2;
            let phi = d[1].atan2(d[0]);
            let to = bearing + ((phi - bearing) / quarter).round() * quarter - phi;
            turn += to.clamp(-0.6, 0.6);
        }
        let t = if planned {
            0.42 + 0.16 * draw(h, 4)
        } else {
            0.3 + 0.4 * draw(h, 5)
        };
        let kinked = !planned && draw(h, 6) < 0.6;
        // A slight kink part-way along (4°–17°).
        let k = (0.07 + 0.23 * draw(h, 7)) * if draw(h, 8) < 0.5 { -1.0 } else { 1.0 };
        let shift = s11(h ^ 0x4D) * 0.25;
        // The full turn first; gentler ones where it would leave a wedge.
        [1.0, 0.5, 0.0]
            .into_iter()
            .map(|f| {
                let d = rotate(d, turn * f);
                let c = at_share(region, mo.centroid, d, t);
                if !kinked {
                    return Candidate::straight(c, d);
                }
                let (alo, ahi) = extent(region, d);
                let s = shift * (ahi - alo);
                let ck = [c[0] + d[0] * s, c[1] + d[1] * s];
                let (d1, d2) = (rotate(d, -k / 2.0), rotate(d, k / 2.0));
                Candidate {
                    cut: Cut::kinked(ck, d1, d2),
                    kink: Some((ck, d1, d2)),
                    line: (ck, d),
                    strict: true,
                    road: false,
                }
            })
            .collect()
    }

    /// A road crossing the region nearly straight, as a cut.
    fn guide(&self, region: &[Vec<P>], mo: &Moments, short: f64) -> Option<Candidate> {
        let b = bbox(region);
        let mut best: Option<(f64, P, Candidate)> = None;
        for g in self.guides {
            let gb = g.bbox;
            if gb[0] > b[2] || gb[2] < b[0] || gb[1] > b[3] || gb[3] < b[1] {
                continue;
            }
            let Some(st) = stretch(g, b, 8.0) else {
                continue;
            };
            let inside: Vec<P> = st
                .iter()
                .filter(|p| region_contains(region, **p))
                .copied()
                .collect();
            if inside.len() < 3 {
                continue;
            }
            let span = {
                let (a, z) = (inside[0], inside[inside.len() - 1]);
                (z[0] - a[0]).hypot(z[1] - a[1])
            };
            if span < 0.5 * short {
                continue;
            }
            let (c, mut d, dev) = fit(&inside);
            if dev > 0.08 * span + 4.0 {
                continue;
            }
            let run = [
                inside[inside.len() - 1][0] - inside[0][0],
                inside[inside.len() - 1][1] - inside[0][1],
            ];
            if dot(d, run) < 0.0 {
                d = [-d[0], -d[1]];
            }
            let cand = Candidate {
                cut: Cut::along(&st),
                kink: None,
                line: (c, d),
                strict: false,
                road: true,
            };
            let kids = cand.children(region);
            let ok = kids
                .iter()
                .all(|k| moments(k, self.origin).area >= (0.12 * mo.area).max(2.0 * MIN_AREA));
            if !ok {
                continue;
            }
            let better = best.as_ref().is_none_or(|(s, bc, _)| {
                span > *s || (span == *s && (c[0], c[1]) < (bc[0], bc[1]))
            });
            if better {
                best = Some((span, c, cand));
            }
        }
        best.map(|(_, _, c)| c)
    }
}

/// The directions to cut a region across and lengthwise: square to its
/// longest straight side (its frontage on a lane or older boundary), so
/// new hedges meet old ones near right angles, as in real enclosure.
fn frontage_axes(region: &[Vec<P>], e1: P) -> (P, P) {
    // Edge length per 6° bin of direction (mod 180°), as doubled-angle
    // vectors so opposite edges agree.
    const BINS: usize = 30;
    let mut w = [0.0_f64; BINS];
    let mut v = [[0.0_f64; 2]; BINS];
    for piece in region {
        for (i, &a) in piece.iter().enumerate() {
            let b = piece[(i + 1) % piece.len()];
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx.hypot(dy);
            if len < 1e-6 {
                continue;
            }
            let ang = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let k = ((ang / std::f64::consts::PI * BINS as f64) as usize).min(BINS - 1);
            let (s2, c2) = (2.0 * ang).sin_cos();
            w[k] += len;
            v[k] = [v[k][0] + c2 * len, v[k][1] + s2 * len];
        }
    }
    // The heaviest pair of neighbouring bins.
    let k = (0..BINS)
        .max_by(|&a, &b| {
            let s = |i: usize| w[i] + w[(i + 1) % BINS];
            s(a).total_cmp(&s(b)).then(b.cmp(&a))
        })
        .unwrap_or(0);
    let (a, b) = (v[k], v[(k + 1) % BINS]);
    let half = 0.5 * (a[1] + b[1]).atan2(a[0] + b[0]);
    let f = [half.cos(), half.sin()];
    // Across the long axis is whichever of the side and its normal lies
    // nearer to the long axis's normal.
    let n1 = perp(e1);
    if dot(f, n1).abs() >= dot(perp(f), n1).abs() {
        (f, perp(f))
    } else {
        (perp(f), f)
    }
}

/// The distance from `p` to the polyline `cut` and the unit direction of
/// its nearest segment.
fn nearest_on(cut: &[P], p: P) -> (f64, P) {
    let mut best = (f64::INFINITY, [1.0, 0.0]);
    for s in cut.windows(2) {
        let (q, _) = closest_on_segment(p, s[0], s[1]);
        let d = (q[0] - p[0]).hypot(q[1] - p[1]);
        if d < best.0 {
            best = (d, unit(sub(s[1], s[0])));
        }
    }
    best
}

/// Where segment `p`–`q` crosses segment `a`–`b`, and the acute angle
/// between them, degrees; `None` if they do not cross (or run parallel).
fn crossing(p: P, q: P, a: P, b: P) -> Option<(P, f64)> {
    let (r, e) = (sub(q, p), sub(b, a));
    let (lr, le) = (r[0].hypot(r[1]), e[0].hypot(e[1]));
    let den = cross(r, e);
    if lr < 1e-9 || le < 1e-9 || den.abs() < 1e-12 * lr * le {
        return None;
    }
    let ap = sub(a, p);
    let (t, u) = (cross(ap, e) / den, cross(ap, r) / den);
    if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
        return None;
    }
    let c = (dot(r, e) / (lr * le)).abs().min(1.0);
    Some(([p[0] + r[0] * t, p[1] + r[1] * t], c.acos().to_degrees()))
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// The share of its principal-axis bounding box a region fills.
fn fill(region: &[Vec<P>], m: &Moments) -> f64 {
    let e1 = major_axis(m.cov);
    let (a, b) = extent(region, e1);
    let (c, d) = extent(region, perp(e1));
    m.area / ((b - a) * (d - c)).max(1e-9)
}

/// The point on the line through the centroid along `d` moved across to
/// share `t` of the region's extent along `d`'s normal.
fn at_share(region: &[Vec<P>], centroid: P, d: P, t: f64) -> P {
    let n = perp(d);
    let (lo, hi) = extent(region, n);
    let s = lo + t * (hi - lo) - dot(centroid, n);
    [centroid[0] + n[0] * s, centroid[1] + n[1] * s]
}
