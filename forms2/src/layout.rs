use super::*;

pub mod grid;

pub enum Layout {
    Fixed(Size, Point),
    Grid { row: i32, col: i32 },
    Docked {},
}


pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

pub enum VerticalAlignment {
    Top,
    Center,
    Bottom,
    Baseline,
}
