use super::*;

static_assertions::assert_not_impl_any!(ControlState: Send, Sync);

pub struct ControlState {
    pub(crate) stuck: StuckToThread,
    #[allow(dead_code)]
    pub(crate) form: Weak<Form>,
    hwnd: HWND,
}

impl core::fmt::Debug for ControlState {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(fmt, "Control")
    }
}

impl ControlState {
    pub(crate) fn handle(&self) -> HWND {
        self.stuck.check();
        self.hwnd
    }

    pub(crate) fn check_thread(&self) {
        self.stuck.check();
    }

    pub(crate) fn new(form: &Rc<Form>, hwnd: HWND) -> ControlState {
        Self {
            hwnd,
            stuck: StuckToThread::new(),
            form: Rc::downgrade(form),
        }
    }

    pub fn set_tab_stop(&self, value: bool) {
        self.check_thread();
        self.set_window_style_bits(WS_TABSTOP, if value { WS_TABSTOP } else { WINDOW_STYLE(0) })
    }

    pub(crate) fn get_window_style(&self) -> WINDOW_STYLE {
        unsafe { WINDOW_STYLE(GetWindowLongW(self.hwnd, GWL_STYLE) as u32) }
    }

    #[allow(dead_code)]
    pub(crate) fn get_window_style_ex(&self) -> WINDOW_EX_STYLE {
        self.check_thread();
        unsafe { WINDOW_EX_STYLE(GetWindowLongW(self.hwnd, GWL_EXSTYLE) as u32) }
    }

    pub(crate) fn set_window_style(&self, style: WINDOW_STYLE) {
        self.check_thread();
        unsafe {
            SetWindowLongW(self.hwnd, GWL_STYLE, style.0 as i32);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_window_style_ex(&self, style: WINDOW_EX_STYLE) {
        self.check_thread();
        unsafe {
            SetWindowLongW(self.hwnd, GWL_EXSTYLE, style.0 as i32);
        }
    }

    pub(crate) fn set_window_style_bits(&self, mask: WINDOW_STYLE, value: WINDOW_STYLE) {
        self.check_thread();
        assert!(value.0 & !mask.0 == 0);
        let style = self.get_window_style();
        let new_style = (style.0 & !mask.0) | value.0;
        self.set_window_style(WINDOW_STYLE(new_style));
    }

    #[allow(dead_code)]
    pub(crate) fn set_window_style_ex_bits(&self, mask: WINDOW_EX_STYLE, value: WINDOW_EX_STYLE) {
        self.check_thread();
        assert!(value.0 & !mask.0 == 0);
        let style = self.get_window_style_ex();
        let new_style = (style.0 & !mask.0) | value.0;
        self.set_window_style_ex(WINDOW_EX_STYLE(new_style));
    }

    pub(crate) fn set_window_style_flag(&self, mask: WINDOW_STYLE, value: bool) {
        self.check_thread();
        self.set_window_style_bits(mask, if value { mask } else { WINDOW_STYLE(0) });
    }

    /// Converts a client-area coordinate of a specified point to screen coordinates.
    ///
    /// See <https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-clienttoscreen>
    pub fn client_to_screen(&self, client_point: POINT) -> POINT {
        self.check_thread();
        unsafe {
            let mut result: POINT = client_point;
            if ClientToScreen(self.handle(), &mut result).into() {
                result
            } else {
                // Wrong, but whatevs.
                client_point
            }
        }
    }

    pub fn set_rect(&self, rect: &Rect) {
        self.check_thread();
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND(0), // insert after
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SET_WINDOW_POS_FLAGS(0),
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
