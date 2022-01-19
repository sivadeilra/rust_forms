use super::*;

pub struct GridLayout {
    pub cols: GridLayoutAxis,
    pub rows: GridLayoutAxis,
    pub items: Vec<GridItem>,
}

pub struct GridItem {
    pub item: LayoutItem,
    pub col: u16,
    pub row: u16,
    pub col_span: u16,
    pub row_span: u16,
}

pub struct GridLayoutAxis {
    pub dims: Vec<GridDim>,
    pub start_margin: i32,
    pub end_margin: i32,
    pub padding: i32,
}

pub enum GridDim {
    Fixed(i32),
    Auto,
    Proportional(f32),
}

impl GridLayout {
    #[cfg(todo)]
    pub fn add_control(
        &mut self,
        col: u16,
        col_span: u16,
        row: u16,
        row_span: u16,
        control: &Control,
    ) {
    }
}
