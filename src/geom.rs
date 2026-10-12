use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Pos { x, y, z }
    }

    pub fn offset(self, dx: i32, dy: i32, dz: i32) -> Pos {
        Pos::new(self.x + dx, self.y + dy, self.z + dz)
    }

    pub fn up(self) -> Pos {
        self.offset(0, 0, 1)
    }

    pub fn down(self) -> Pos {
        self.offset(0, 0, -1)
    }
}

pub const DIRS8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

pub const DIRS4: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

/// Every position in the axis-aligned box spanned by two corners (inclusive).
pub fn box_iter(a: Pos, b: Pos) -> impl Iterator<Item = Pos> {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    let (z0, z1) = (a.z.min(b.z), a.z.max(b.z));
    (z0..=z1).flat_map(move |z| {
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| Pos::new(x, y, z)))
    })
}
