// https://docs.microsoft.com/en-us/windows/win32/controls/status-bars

use super::*;

pub struct StatusBar {
    control: ControlState,
}

impl core::ops::Deref for StatusBar {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

const STATUSCLASSNAME: &str = "msctls_statusbar32";

impl StatusBar {
    pub fn new(form: &Rc<Form>) -> Rc<Self> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate(STATUSCLASSNAME);
            let ex_style = 0;

            let text = U16CString::from_str_truncate("");

            let hwnd = CreateStatusWindowW(
                (WS_CHILD | WS_VISIBLE).0 as i32,
                PCWSTR::from_raw(text.as_ptr()),
                form.handle(),
                0,
            )
            .unwrap();

            let state: Rc<StatusBar> = Rc::new(StatusBar {
                control: ControlState::new(hwnd),
            });

            _ = SendMessageW(
                state.handle(),
                SB_SIMPLE,
                Some(WPARAM(1)), // set it to simple mode.
                Some(LPARAM(0)),
            );

            form.invalidate_layout();
            state
        }
    }

    pub fn set_status(&self, s: &str) {
        unsafe {
            let ws = U16CString::from_str_truncate(s);

            _ = SendMessageW(
                self.handle(),
                SB_SETTEXT,
                Some(WPARAM(SB_SIMPLEID as _)),
                Some(LPARAM(ws.as_ptr() as _)),
            );
        }
    }
}
