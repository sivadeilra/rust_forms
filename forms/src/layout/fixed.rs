use super::*;

pub struct FixedLayout {
    pub items: Vec<FixedItem>,
}

/// An item in a Fixed layout.
pub struct FixedItem {
    pub item: Item,
    pub pos: Point,
    pub size: Size,
}
