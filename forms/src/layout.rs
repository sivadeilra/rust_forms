use std::rc::Rc;

pub mod fixed;
pub mod grid;

pub enum Layout {
    /// The application must explicitly set the position and size of each
    /// control.
    Fixed(fixed::FixedLayout),

    /// Child nodes are placed in a grid. The application specifies the size
    /// of each row and column of the grid, and places controls into specific
    /// rows and columns. Grid layout then computes the size and position of
    /// each control.
    Grid(grid::GridLayout),

    /// Child nodes are stacked horizontally (or vertically). Their position
    /// depends on their size. Extra space is left unused.
    Stack,
}

pub trait ItemT {
    fn get_min_max_size(&self) -> (Size, Size);
    fn set_size_pos(&self, size: Size, pos: Point);
}

/// An item that participates in a Layout.
pub enum Item {
    /// The item is a nested layout.
    Layout(Box<Layout>),
    /// The item is a control.
    Control(Rc<dyn ItemT>),
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Point(pub i32, pub i32);

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Size(pub i32, pub i32);

impl Layout {
    pub fn set_size_pos(&mut self, size: Size, pos: Point) {
        match self {
            Self::Fixed(_) => {
                // Nothing. That's the whole idea.
            }

            Self::Grid(grid) => grid.set_size_pos(size, pos),
            Stack => todo!(),
        }
    }
}
