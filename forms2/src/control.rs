use super::*;

pub struct Control {
    pub(crate) state: Rc<UnsafeCell<ControlState>>,
}

static_assertions::assert_not_impl_any!(Control: Send, Sync);

pub(crate) struct ControlState {
    pub(crate) layout: ControlLayout,
    pub(crate) form: Weak<FormState>,
    // pub(crate) parent: Weak<ControlState>,
    pub(crate) controls: Vec<Rc<ControlState>>,
    pub(crate) handle: HWND,
}

pub(crate) struct ControlLayout {
    pub(crate) grid_row: i32,
    pub(crate) grid_row_span: i32,
    pub(crate) grid_col: i32,
    pub(crate) grid_col_span: i32,
    pub(crate) grid_horizontal_alignment: HorizontalAlignment,
    pub(crate) grid_vertical_alignment: VerticalAlignment,
}

impl Default for ControlLayout {
    fn default() -> Self {
        Self {
            grid_row: 0,
            grid_row_span: 0,
            grid_col: 0,
            grid_col_span: 0,
            grid_horizontal_alignment: HorizontalAlignment::Left,
            grid_vertical_alignment: VerticalAlignment::Top,
        }
    }
}

impl Control {
    pub(crate) fn handle(&self) -> HWND {
        unsafe { self.state_ref().handle }
    }

    unsafe fn state_ref(&self) -> &ControlState {
        &*self.state.get()
    }

    unsafe fn state_mut(&self) -> &mut ControlState {
        &mut *self.state.get()
    }

    pub fn set_fixed_layout(self, size: Size, pos: Point) {
        unsafe {
            let state = self.state_mut();
        }
    }

    pub fn set_min_size(&self, min_size: Option<Size>) {}
    pub fn set_max_size(&self, max_size: Option<Size>) {}

    /// Sets the grid layout parameters for this control. This is only meaningful
    /// if the layout context is Grid.
    pub fn set_grid_layout(&self, row: i32, row_span: i32, col: i32, col_span: i32) {
        unsafe {
            let state = self.state_mut();
            state.layout.grid_row = row;
            state.layout.grid_row_span = row_span;
            state.layout.grid_col = col;
            state.layout.grid_col_span = col_span;
            if let Some(form) = state.form.upgrade() {
                form.invalidate_layout();
            }
        }
    }

    /// Sets the grid layout parameters for this control. This is only meaningful
    /// if the layout context is Grid.
    pub fn set_grid_alignment(&self, horizontal: HorizontalAlignment, vertical: VerticalAlignment) {
        unsafe {
            let state = self.state_mut();
            state.layout.grid_horizontal_alignment = horizontal;
            state.layout.grid_vertical_alignment = vertical;
            if let Some(form) = state.form.upgrade() {
                form.invalidate_layout();
            }
        }
    }
}

impl ControlState {
    pub(crate) fn get_window_style(&self) -> u32 {
        unsafe { GetWindowLongW(self.handle, GWL_STYLE) as u32 }
    }

    pub(crate) fn get_window_style_ex(&self) -> u32 {
        unsafe {
            let value = GetWindowLongW(self.handle, GWL_EXSTYLE) as u32;
            trace!("get window style ex: 0x{:x}", value);
            value
        }
    }

    pub(crate) fn set_window_style(&self, style: u32) {
        unsafe {
            SetWindowLongW(self.handle, GWL_STYLE, style as _);
        }
    }

    pub(crate) fn set_window_style_ex(&self, style: u32) {
        unsafe {
            trace!("set window style ex: 0x{:x}", style);
            SetWindowLongW(self.handle, GWL_EXSTYLE, style as _);
        }
    }

    pub(crate) fn set_window_style_bits(&self, mask: u32, value: u32) {
        assert!(value & !mask == 0);
        unsafe {
            let style = self.get_window_style();
            let new_style = (style & !mask) | value;
            self.set_window_style(new_style);
        }
    }

    pub(crate) fn set_window_style_ex_bits(&self, mask: u32, value: u32) {
        assert!(value & !mask == 0);
        unsafe {
            let style = self.get_window_style_ex();
            let new_style = (style & !mask) | value;
            self.set_window_style_ex(new_style);
        }
    }

    pub(crate) fn set_window_style_flag(&self, mask: u32, value: bool) {
        self.set_window_style_bits(mask, if value { mask } else { 0 });
    }
}
