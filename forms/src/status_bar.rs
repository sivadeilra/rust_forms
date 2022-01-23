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

            let hwnd = CreateStatusWindowW((WS_CHILD | WS_VISIBLE) as i32, "", form.handle(), 0);
            if hwnd == 0 {
                panic!("failed to create StatusBar");
            }

            let state: Rc<StatusBar> = Rc::new(StatusBar {
                control: ControlState::new(form, hwnd),
            });
            form.invalidate_layout();
            state
        }
    }

    pub fn set_status(&self, s: &str) {
        set_window_text(self.handle(), s);
    }
}

impl NotifyHandlerTrait for StatusBar {
    unsafe fn wm_notify(&self, control_id: WPARAM, nmhdr: *mut NMHDR) -> NotifyResult {
        NotifyResult::NotConsumed
    }
}
