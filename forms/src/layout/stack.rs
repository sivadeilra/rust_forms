use super::*;

#[derive(Debug)]
pub struct StackLayout {
    pub items: Vec<LayoutItem>,
    pub lead_margin: i32,
    pub tail_margin: i32,
    // width/height of each item
    pub pitch: i32,
    /// distance between each
    pub padding: i32,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Copy)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl StackLayout {
    pub(crate) fn place(
        &self,
        placer: &mut dyn LayoutPlacer,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        match self.orientation {
            Orientation::Vertical => {
                let mut item_y = y + self.lead_margin;
                for item in self.items.iter() {
                    let item_y_start = item_y;
                    item_y += self.pitch;
                    item.place(placer, x, item_y_start, width, self.pitch);
                    item_y += self.padding;
                }
            }
            Orientation::Horizontal => {
                let mut item_x = self.lead_margin;
                for item in self.items.iter() {
                    let item_x_start = item_x;
                    item_x += self.pitch;
                    item.place(placer, item_x_start, y, self.pitch, height);
                    item_x += self.padding;
                }
            }
        }
    }

    pub(crate) fn min_size(&self) -> (i32, i32) {
        if self.items.is_empty() {
            return (0, 0);
        }

        let min_along = self.lead_margin
            + self.items.len() as i32 * self.pitch
            + (self.items.len() as i32 - 1) * self.padding
            + self.tail_margin;
        match self.orientation {
            Orientation::Vertical => (0, min_along),
            Orientation::Horizontal => (min_along, 0),
        }
    }
}
