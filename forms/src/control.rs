use super::*;

static_assertions::assert_not_impl_any!(ControlState: Send, Sync);

pub struct ControlState {
    pub(crate) layout: RefCell<ControlLayout>,
    pub(crate) form: Weak<Form>,
    pub(crate) handle: HWND,
}

impl core::fmt::Debug for ControlState {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        // use core::fmt::Write;
        write!(fmt, "Control")?;
        Ok(())
    }
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

impl ControlState {
    pub(crate) fn handle(&self) -> HWND {
        self.handle
    }

    pub fn set_tab_stop(&self, value: bool) {
        self.set_window_style_bits(WS_TABSTOP, if value { WS_TABSTOP } else { 0 })
    }

    pub fn set_fixed_layout(self, size: Size, pos: Point) {}

    pub fn set_min_size(&self, min_size: Option<Size>) {}
    pub fn set_max_size(&self, max_size: Option<Size>) {}

    /// Sets the grid layout parameters for this control. This is only meaningful
    /// if the layout context is Grid.
    pub fn set_grid_layout(&self, row: i32, row_span: i32, col: i32, col_span: i32) {
        let mut layout = self.layout.borrow_mut();
        layout.grid_row = row;
        layout.grid_row_span = row_span;
        layout.grid_col = col;
        layout.grid_col_span = col_span;
        drop(layout);

        if let Some(form) = self.form.upgrade() {
            form.invalidate_layout();
        }
    }

    /// Sets the grid layout parameters for this control. This is only meaningful
    /// if the layout context is Grid.
    pub fn set_grid_alignment(&self, horizontal: HorizontalAlignment, vertical: VerticalAlignment) {
        let mut layout = self.layout.borrow_mut();
        layout.grid_horizontal_alignment = horizontal;
        layout.grid_vertical_alignment = vertical;
        drop(layout);

        if let Some(form) = self.form.upgrade() {
            form.invalidate_layout();
        }
    }

    pub(crate) fn get_window_style(&self) -> u32 {
        unsafe { GetWindowLongW(self.handle, GWL_STYLE) as u32 }
    }

    #[allow(dead_code)]
    pub(crate) fn get_window_style_ex(&self) -> u32 {
        unsafe {
            let value = GetWindowLongW(self.handle, GWL_EXSTYLE) as u32;
            trace!("get window style ex: 0x{:x}", value);
            value
        }
    }

    pub(crate) fn set_window_style(&self, style: u32) {
        unsafe {
            trace!("setting window style: 0x{:x}", style);
            SetWindowLongW(self.handle, GWL_STYLE, style as _);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_window_style_ex(&self, style: u32) {
        unsafe {
            trace!("set window style ex: 0x{:x}", style);
            SetWindowLongW(self.handle, GWL_EXSTYLE, style as _);
        }
    }

    pub(crate) fn set_window_style_bits(&self, mask: u32, value: u32) {
        assert!(value & !mask == 0);
        let style = self.get_window_style();
        let new_style = (style & !mask) | value;
        self.set_window_style(new_style);
    }

    #[allow(dead_code)]
    pub(crate) fn set_window_style_ex_bits(&self, mask: u32, value: u32) {
        assert!(value & !mask == 0);
        let style = self.get_window_style_ex();
        let new_style = (style & !mask) | value;
        self.set_window_style_ex(new_style);
    }

    pub(crate) fn set_window_style_flag(&self, mask: u32, value: bool) {
        self.set_window_style_bits(mask, if value { mask } else { 0 });
    }

    /// Converts a client-area coordinate of a specified point to screen coordinates.
    ///
    /// See <https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-clienttoscreen>
    pub fn client_to_screen(&self, client_point: POINT) -> POINT {
        unsafe {
            let mut result: POINT = client_point.clone();
            if ClientToScreen(self.handle(), &mut result).into() {
                result
            } else {
                // Wrong, but whatevs.
                return client_point;
            }
        }
    }

    pub fn set_rect(&self, rect: &Rect) {
        unsafe {
            SetWindowPos(
                self.handle,
                0, // insert after
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                0,
            );
        }
    }
}

#[derive(Default)]
pub struct CreateControlOptions {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
