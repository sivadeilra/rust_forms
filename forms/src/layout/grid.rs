use super::*;

#[derive(Debug)]
pub struct GridLayout {
    pub cols: GridAxis,
    pub rows: GridAxis,
    pub items: Vec<GridItem>,
}

/// Describes the horizontal (or vertical) axis of a GridLayout.
#[derive(Debug, Default)]
pub struct GridAxis {
    pub cells: Vec<GridAxisCell>,
    // Spacing between each row (column).
    pub padding: i32,
    pub lead_margin: i32,
    pub tail_margin: i32,
}

/// Describes one span within a horizontal or vertical axis of a GridLayout.
#[derive(Debug)]
pub struct GridAxisCell {
    pub size: CellSize,
    pub lead_margin: i32,
    pub tail_margin: i32,
}

impl GridAxisCell {
    pub fn auto(min: i32) -> Self {
        Self {
            lead_margin: 0,
            tail_margin: 0,
            size: CellSize::Scaled {
                scale: 1.0,
                min,
                max: i32::MAX,
            },
        }
    }

    pub fn scaled(scale: f32, min: i32) -> Self {
        Self {
            lead_margin: 0,
            tail_margin: 0,
            size: CellSize::Scaled {
                scale,
                min,
                max: i32::MAX,
            },
        }
    }

    pub fn fixed(size: i32) -> Self {
        Self {
            lead_margin: 0,
            tail_margin: 0,
            size: CellSize::Fixed(size),
        }
    }
}

#[derive(Debug)]
pub enum CellSize {
    Fixed(i32),
    Scaled { scale: f32, min: i32, max: i32 },
}

/// A single item placed into a Grid layout.
#[derive(Debug)]
pub struct GridItem {
    pub col: u16,
    pub row: u16,
    pub col_span: u16,
    pub row_span: u16,
    pub item: LayoutItem,
}

impl GridItem {
    pub fn new_spanned(
        row: u16,
        col: u16,
        row_span: u16,
        col_span: u16,
        item: LayoutItem,
    ) -> GridItem {
        GridItem {
            col,
            row,
            col_span,
            row_span,
            item,
        }
    }
    pub fn new(row: u16, col: u16, item: LayoutItem) -> GridItem {
        GridItem {
            col,
            row,
            col_span: 1,
            row_span: 1,
            item,
        }
    }
}

#[derive(Debug)]
struct GridAxisPlacement {
    start: i32,
    end: i32,
}

impl GridAxis {
    // Returns (min_size, num_scaled, scale_sum)
    pub(crate) fn min_size(&self) -> (i32, usize, f32) {
        // First, measure the minimum of the placement.
        let mut min_width = self.lead_margin;
        let mut scale_sum: f32 = 0.0;
        let mut num_scaled: usize = 0;
        for (i, c) in self.cells.iter().enumerate() {
            if i > 0 {
                min_width += self.padding;
            }
            min_width += c.lead_margin;
            match &c.size {
                CellSize::Fixed(cell_size) => {
                    min_width += cell_size;
                }
                CellSize::Scaled { scale, min, max } => {
                    assert!(*scale >= 0.0);
                    assert!(min <= max);
                    min_width += *min;
                    scale_sum += *scale;
                    num_scaled += 1;
                }
            }
            min_width += c.tail_margin;
        }
        min_width += self.tail_margin;

        (min_width, num_scaled, scale_sum)
    }

    fn place(&self, size: i32) -> Vec<GridAxisPlacement> {
        assert!(size >= 0);

        debug!("GridAxis::place: {:?}", self);

        if self.cells.is_empty() {
            return Vec::new();
        }

        let (min_width, num_scaled, scale_sum) = self.min_size();

        let mut placements = Vec::with_capacity(self.cells.len());

        if num_scaled > 0 && scale_sum <= 0.0 {
            warn!("scale values are bad");
            return Vec::new();
        }

        // Compute how much "extra" space we have.
        let extra = size - size.min(min_width);
        assert!(extra >= 0);

        trace!(
            "min_width = {}, scale_sum = {}, extra = {}",
            min_width,
            scale_sum,
            extra
        );

        // Build the placement.
        let mut x = self.lead_margin;
        let mut extra_available = extra;
        for (i, c) in self.cells.iter().enumerate() {
            if i > 0 {
                x += self.padding;
            }
            x += c.lead_margin;
            let cell_start = x;

            let c_width;
            match &c.size {
                CellSize::Fixed(cell_size) => {
                    c_width = *cell_size;
                }
                CellSize::Scaled { scale, min, max } => {
                    // How much of the "extra" do we use for this one?
                    let proportion = *scale / scale_sum;
                    let space_assigned =
                        min + ((extra as f32 * proportion) as i32).max(extra_available);
                    // space_assigned = space_assigned.max(*min);
                    // space_assigned = space_assigned.min(*max);
                    extra_available -= space_assigned;
                    c_width = space_assigned;
                }
            }

            x += c_width;

            let cell_end = x;

            placements.push(GridAxisPlacement {
                start: cell_start,
                end: cell_end,
            });

            x += c.tail_margin;
        }

        trace!("placements: {:?}", placements);

        placements
    }
}

impl GridLayout {
    pub(crate) fn place(
        &self,
        placer: &mut dyn LayoutPlacer,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        trace!("GridLayout: row_placement:");
        let row_placement = self.rows.place(height);
        trace!("GridLayout: col_placement:");
        let col_placement = self.cols.place(width);

        fn get_range(
            which: &str,
            coord: i32, // starting coordinate along the axis being laid out
            placements: &[GridAxisPlacement],
            i: u16,
            span: u16,
        ) -> (i32, i32) {
            let i = i as usize;
            let span = span as usize;
            if i < placements.len() && i + span <= placements.len() {
                (
                    coord + placements[i].start,
                    coord + placements[i + span - 1].end,
                )
            } else {
                warn!("grid placement out of range: {} {} span {}", which, i, span);
                (0, 0)
            }
        }

        for item in self.items.iter() {
            let col_range = get_range("col", x, &col_placement, item.col, item.col_span);
            let row_range = get_range("row", y, &row_placement, item.row, item.row_span);

            let item_x = col_range.0;
            let item_y = row_range.0;
            let item_width = col_range.1 - col_range.0;
            let item_height = row_range.1 - row_range.0;

            trace!(
                "placing item: {},{} + {},{}",
                item_x,
                item_y,
                item_width,
                item_height
            );

            item.item.place(
                placer,
                col_range.0,
                row_range.0,
                col_range.1 - col_range.0,
                row_range.1 - row_range.0,
            );
        }
    }

    pub(crate) fn get_min_size(&self) -> (i32, i32) {
        let row_min_size = self.rows.min_size().0;
        let col_min_size = self.cols.min_size().0;
        (col_min_size, row_min_size)
    }
}
