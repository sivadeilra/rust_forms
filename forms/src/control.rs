use super::*;

pub struct Control {
    handle: WindowHandle,
}

impl Control {
    pub fn from_handle(handle: WindowHandle) -> Self {
        Self {
            handle,
        }
    }

    pub fn set_grid_layout(&self, col: u16, col_span: u16, row: u16, row_span: u16) {
    }

    pub fn set_fixed_layout(&self, pos: Point) {
    }

    pub fn set_size(&self, size: Size) {
    }
}

impl core::ops::Deref for Control {
    type Target = WindowHandle;
    fn deref(&self) -> &WindowHandle {
        &self.handle
    }
}