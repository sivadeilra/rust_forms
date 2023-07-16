use super::*;

pub mod grid;
pub mod stack;

pub use stack::{Orientation, StackLayout};

#[derive(Debug)]
pub enum Layout {
    /// The application must explicitly set the position and size of each
    /// control.
    #[cfg(nope)]
    Fixed(fixed::FixedLayout),

    /// Child nodes are placed in a grid. The application specifies the size
    /// of each row and column of the grid, and places controls into specific
    /// rows and columns. Grid layout then computes the size and position of
    /// each control.
    Grid(grid::GridLayout),

    /// Child nodes are stacked horizontally (or vertically). Their position
    /// depends on their size. Extra space is left unused.
    Stack(stack::StackLayout),
}

impl Layout {
    pub(crate) fn place(
        &self,
        placer: &mut dyn LayoutPlacer,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        match self {
            Self::Grid(grid) => grid.place(placer, x, y, width, height),
            Self::Stack(stack) => stack.place(placer, x, y, width, height),
        }
    }

    pub(crate) fn get_min_size(&self) -> (i32, i32) {
        match self {
            Self::Grid(grid) => grid.get_min_size(),
            Self::Stack(stack) => stack.min_size(),
        }
    }
}

/// An item that participates in a Layout.
pub enum LayoutItem {
    /// The item is a nested layout.
    Layout(Box<Layout>),
    /// The item is a control.
    Control(Rc<dyn core::ops::Deref<Target = ControlState>>),
}

impl core::fmt::Debug for LayoutItem {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        // use core::fmt::Write;
        write!(fmt, "LayoutItem")?;
        match self {
            Self::Layout(nested_layout) => {
                fmt.debug_struct("Layout")
                    .field("self", nested_layout)
                    .finish()?;
            }
            Self::Control(_) => {
                write!(fmt, "Control")?;
            }
        }
        Ok(())
    }
}

impl LayoutItem {
    pub(crate) fn place(
        &self,
        placer: &mut dyn LayoutPlacer,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        match self {
            Self::Layout(nested_layout) => nested_layout.place(placer, x, y, width, height),
            Self::Control(control) => {
                placer.place_control(control, x, y, width, height);
            }
        }
    }
}

pub(crate) trait LayoutPlacer {
    fn place_control(&mut self, control: &ControlState, x: i32, y: i32, width: i32, height: i32);
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

pub(crate) struct DeferredLayoutPlacer {
    op: DeferWindowPosOp,
}

impl DeferredLayoutPlacer {
    pub(crate) fn new(n: u32) -> Self {
        Self {
            op: DeferWindowPosOp::begin(10).unwrap(),
        }
    }
}

impl LayoutPlacer for DeferredLayoutPlacer {
    fn place_control(&mut self, control: &ControlState, x: i32, y: i32, width: i32, height: i32) {
        self.op
            .defer(control.handle(), HWND(0), x, y, width, height, SWP_NOZORDER);
    }
}
