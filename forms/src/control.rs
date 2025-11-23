use super::*;

static_assertions::assert_not_impl_any!(ControlState: Send, Sync);

pub struct ControlState {
    pub(crate) stuck: StuckToThread,
    pub(crate) hwnd: Cell<HWND>,
}

impl Drop for ControlState {
    fn drop(&mut self) {
        self.stuck.check();

        unsafe {
            _ = DestroyWindow(self.hwnd.get());
        }
    }
}

impl core::fmt::Debug for ControlState {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(fmt, "Control")
    }
}

impl ControlState {
    pub(crate) fn handle(&self) -> HWND {
        self.stuck.check();
        self.hwnd.get()
    }

    pub(crate) fn check_thread(&self) {
        self.stuck.check();
    }

    pub(crate) fn new(hwnd: HWND) -> Rc<Self> {
        Rc::new(Self {
            hwnd: Cell::new(hwnd),
            stuck: StuckToThread::new(),
        })
    }

    pub fn set_tab_stop(&self, value: bool) {
        self.check_thread();
        self.set_window_style_bits(WS_TABSTOP, if value { WS_TABSTOP } else { WINDOW_STYLE(0) })
    }

    pub(crate) fn get_window_style(&self) -> WINDOW_STYLE {
        unsafe { WINDOW_STYLE(GetWindowLongW(self.hwnd.get(), GWL_STYLE) as u32) }
    }

    pub(crate) fn get_window_style_flag(&self, flag: WINDOW_STYLE) -> bool {
        let styles = self.get_window_style();
        styles.0 & flag.0 != 0
    }

    #[allow(dead_code)]
    pub(crate) fn get_window_style_ex(&self) -> WINDOW_EX_STYLE {
        self.check_thread();
        unsafe { WINDOW_EX_STYLE(GetWindowLongW(self.hwnd.get(), GWL_EXSTYLE) as u32) }
    }

    pub(crate) fn set_window_style(&self, style: WINDOW_STYLE) {
        self.check_thread();
        unsafe {
            SetWindowLongW(self.hwnd.get(), GWL_STYLE, style.0 as i32);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_window_style_ex(&self, style: WINDOW_EX_STYLE) {
        self.check_thread();
        unsafe {
            SetWindowLongW(self.hwnd.get(), GWL_EXSTYLE, style.0 as i32);
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
            _ = SetWindowPos(
                self.hwnd.get(),
                None, // insert after
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SET_WINDOW_POS_FLAGS(0),
            );
        }
    }

    pub fn get_client_rect(&self) -> RECT {
        unsafe {
            let mut client_rect: RECT = zeroed();
            _ = GetClientRect(self.hwnd.get(), &mut client_rect);
            client_rect
        }
    }

    pub fn set_visible(&self, value: bool) {
        self.set_window_style_flag(WS_VISIBLE, value);
    }

    pub fn show(&self) {
        unsafe {
            _ = SetWindowPos(
                self.hwnd.get(),
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOZORDER | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn hide(&self) {
        unsafe {
            _ = SetWindowPos(
                self.hwnd.get(),
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOZORDER | SWP_HIDEWINDOW,
            );
        }
    }

    pub fn invalidate_all(&self) {
        unsafe {
            _ = InvalidateRect(Some(self.hwnd.get()), None, true);
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.handle(), text);
    }

    pub fn set_font(&self, font: &Font) {
        unsafe {
            SendMessageW(
                self.handle(),
                WM_SETFONT,
                Some(WPARAM(font.hfont.0 as usize)),
                Some(LPARAM(1)),
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

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct ControlId(pub u16);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Command(pub u32);

macro_rules! commands {
    (
        $($value:expr, $name:ident;)*
    ) => {
        impl Command {
            $(
                #[allow(non_upper_case_globals)]
                pub const $name: Command = Command($value);
            )*
        }

        impl std::fmt::Debug for Command {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $( Self::$name => f.write_str(stringify!($name)), )*
                    _ => write!(f, "??(0x{:04x})", self.0)
                }
            }
        }
    }
}

use windows::Win32::UI::WindowsAndMessaging as wm;

commands! {
    wm::BN_CLICKED, ButtonClicked; // 0
    wm::BN_PAINT, ButtonPaint; // 1
    wm::BN_HILITE, ButtonHilite; // 2
    wm::BN_DISABLE, ButtonDisable; // 4
    wm::BN_DBLCLK, ButtonDoubleClicked; // 5
    wm::BN_KILLFOCUS, ButtonKillFocus; // 7
}

#[macro_export]
macro_rules! control_ids {
    ($($name:ident,)*) => {

        #[allow(non_camel_case_types)]
        #[repr(u16)]
        enum __ControlIds {
            $( $name, )*
        }

        $(
            pub const $name: ControlId = ControlId(__ControlIds::$name as u16);
        )*
    }
}
