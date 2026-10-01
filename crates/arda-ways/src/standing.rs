//! Standing water already in the window (lakes and sea from the refined
//! layout): roads never pave it, channels never repaint it, and a crossing
//! whose foot lies in it gets no synthetic river (review round 2 #33).

use crate::plan::Window;
use arda_tactical::TacticalLayout;

/// The window's squares that already hold water, row-major.
#[derive(Debug, Clone, Default)]
pub struct Standing {
    gx0: i64,
    gy0: i64,
    w: i64,
    h: i64,
    mask: Vec<bool>,
}

impl Standing {
    /// The squares of `layout` (whose north-west square is the window's)
    /// with a water depth before any way is applied.
    #[must_use]
    pub fn of(layout: &TacticalLayout, win: Window) -> Self {
        Self {
            gx0: win.gx0,
            gy0: win.gy0,
            w: i64::from(layout.width),
            h: i64::from(layout.height),
            mask: layout
                .squares
                .iter()
                .map(|s| s.water_depth_ft > 0)
                .collect(),
        }
    }

    /// Whether global square `(gx, gy)` holds standing water; squares
    /// outside the window do not.
    #[must_use]
    pub fn at(&self, gx: i64, gy: i64) -> bool {
        let (x, y) = (gx - self.gx0, gy - self.gy0);
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return false;
        }
        usize::try_from(y * self.w + x)
            .ok()
            .and_then(|i| self.mask.get(i).copied())
            .unwrap_or(false)
    }
}
