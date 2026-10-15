//! What a solved building's rooms hold, for review and tests: each room's
//! tag, rectangle, blocking squares and furniture, and which of its walls
//! a piece lines (a library's shelves on every wall, logic/10 §town-wfc).

use super::indoor::Plan;

/// One room's furnishing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomStats {
    /// Zone tag (`great_hall`, `reading_room`, …).
    pub tag: &'static str,
    /// Canonical rectangle `[x0, y0, x1, y1]`.
    pub rect: [i64; 4],
    /// Floor squares.
    pub area: i64,
    /// Squares a piece blocks.
    pub blocking: usize,
    /// Every piece's name and anchor `(x, y, orientation)`.
    pub pieces: Vec<(&'static str, i64, i64, u8)>,
}

impl RoomStats {
    /// How many pieces named `name` the room holds.
    #[must_use]
    pub fn count(&self, name: &str) -> usize {
        self.pieces.iter().filter(|p| p.0 == name).count()
    }

    /// How many of the room's four walls a piece named `name` stands
    /// against (its anchor row or column on the wall's side).
    #[must_use]
    pub fn walls_lined(&self, name: &str) -> usize {
        let [x0, y0, x1, y1] = self.rect;
        let mut sides = [false; 4];
        for &(n, x, y, o) in &self.pieces {
            if n != name {
                continue;
            }
            // Wall pieces face away from their wall: orientation 0 stands
            // on the north wall, 1 on the east, 2 south, 3 west.
            let on = match o {
                0 => y == y0,
                1 => x + 1 == x1,
                2 => y + 1 == y1,
                _ => x == x0,
            };
            if on {
                sides[usize::from(o % 4)] = true;
            }
        }
        sides.iter().filter(|s| **s).count()
    }
}

/// Every room of a solved building, in layout order.
#[must_use]
pub fn rooms(p: &Plan<'_>) -> Vec<RoomStats> {
    let (v, f) = &p.furnished;
    let w = p
        .layout
        .rooms
        .iter()
        .map(|r| r.rect[2])
        .max()
        .unwrap_or(1)
        .max(1);
    p.layout
        .rooms
        .iter()
        .enumerate()
        .map(|(ri, r)| {
            let mut s = RoomStats {
                tag: p.prog.zones.get(r.zone).map_or("", |z| z.tag),
                rect: r.rect,
                area: r.area(),
                blocking: 0,
                pieces: Vec::new(),
            };
            for (i, &t) in f.tiles.iter().enumerate() {
                if p.layout.cell_room.get(i) != Some(&ri) {
                    continue;
                }
                let t = usize::from(t);
                if !v.walkable(t) {
                    s.blocking += 1;
                }
                let Some(tile) = v.tiles.get(t) else {
                    continue;
                };
                // Pieces 0 and 1 are the floor and the doorway.
                if tile.anchor() && tile.piece > 1 {
                    let i = i64::try_from(i).unwrap_or(0);
                    s.pieces
                        .push((v.pieces[tile.piece].name, i % w, i / w, tile.o));
                }
            }
            s
        })
        .collect()
}
