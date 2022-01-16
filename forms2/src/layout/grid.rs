use super::*;

pub struct Grid {
    pub rows: GridAxis,
    pub cols: GridAxis,
}

pub struct GridAxis {
    pub items: Vec<GridAxisItem>,
    pub start_margin: i32,
    pub end_margin: i32,
}

pub struct GridAxisItem {
    pub size: ItemSize,
    pub min_size: i32,
    pub max_size: i32,
}

pub enum ItemSize {
    Fixed(i32),
    Scaled(f32),
}



