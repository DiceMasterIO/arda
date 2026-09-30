//! Tier parameters: town radius from population density, street widths and
//! burgage plot sizes. Widths follow surveyed medieval burgages (1–2 perches,
//! 5–10 m, in towns) and village tofts (12–25 m).

use super::grid::SQUARE_M;
use crate::site::{SettlementFunction, Tier, TownSite};

/// Planner parameters for one site.
#[derive(Debug, Clone)]
pub struct Params {
    /// Radius of the built core, metres.
    pub r_core: f64,
    /// Main street width, metres.
    pub main_w: f64,
    /// Lane width, metres.
    pub lane_w: f64,
    /// Plot frontage range in squares.
    pub plot_w: (i32, i32),
    /// Frontage range for plots on the market square.
    pub square_plot_w: (i32, i32),
    /// Plot depth range in squares.
    pub depth: (i32, i32),
    /// Building setback range from the plot front, squares.
    pub setback: (i32, i32),
    /// Free squares left beside a building on its plot.
    pub side_gap: i32,
    /// Whether the town is walled.
    pub walled: bool,
    /// Whether a castle ward is built at the focal point.
    pub castle: bool,
    /// Whether back lanes run behind the main-street plots.
    pub back_lanes: bool,
    /// Spacing of cross alleys along main streets, metres.
    pub alley_spacing: (f64, f64),
    /// Market square length and width, metres.
    pub square: (f64, f64),
    /// Whether the square is a green.
    pub green: bool,
}

impl Params {
    /// Parameters for a site, with the core radius scaled by `grow`.
    #[must_use]
    pub fn for_site(site: &TownSite, grow: f64) -> Self {
        let density = match site.tier {
            Tier::City => 230.0,
            Tier::Town => 170.0,
            Tier::Village => 45.0,
            Tier::Hamlet => 14.0,
        };
        let pop = f64::from(site.population.max(12));
        let r_core = (pop / density * 10_000.0 / std::f64::consts::PI).sqrt() * grow;
        let castle =
            site.has(SettlementFunction::Fortress) || site.has(SettlementFunction::Capital);
        let sq = SQUARE_M;
        match site.tier {
            Tier::City => Self {
                r_core,
                main_w: 7.0 * sq,
                lane_w: 3.0 * sq,
                plot_w: (3, 5),
                square_plot_w: (3, 4),
                depth: (16, 24),
                setback: (0, 0),
                side_gap: 0,
                walled: true,
                castle,
                back_lanes: true,
                alley_spacing: (70.0, 110.0),
                square: (110.0, 44.0),
                green: false,
            },
            Tier::Town => Self {
                r_core,
                main_w: 6.0 * sq,
                lane_w: 3.0 * sq,
                plot_w: (4, 6),
                square_plot_w: (3, 5),
                depth: (18, 26),
                setback: (0, 1),
                side_gap: 0,
                walled: true,
                castle,
                back_lanes: true,
                alley_spacing: (80.0, 130.0),
                square: (80.0, 34.0),
                green: false,
            },
            Tier::Village => Self {
                r_core,
                main_w: 5.0 * sq,
                lane_w: 3.0 * sq,
                plot_w: (8, 13),
                square_plot_w: (7, 10),
                depth: (18, 30),
                setback: (1, 3),
                side_gap: 1,
                walled: false,
                castle,
                back_lanes: true,
                alley_spacing: (140.0, 220.0),
                square: (46.0, 30.0),
                green: true,
            },
            Tier::Hamlet => Self {
                r_core,
                main_w: 3.0 * sq,
                lane_w: 2.0 * sq,
                plot_w: (9, 15),
                square_plot_w: (9, 12),
                depth: (14, 24),
                setback: (1, 4),
                side_gap: 2,
                walled: false,
                castle,
                back_lanes: false,
                alley_spacing: (400.0, 600.0),
                square: (22.0, 14.0),
                green: true,
            },
        }
    }

    /// Depth of a standard plot in metres (midpoint of the range).
    #[must_use]
    pub fn depth_m(&self) -> f64 {
        f64::from(self.depth.0 + self.depth.1) * 0.5 * SQUARE_M
    }
}
