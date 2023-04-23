use super::*;

pub struct Label {
    control: ControlState,
    // font: Option<Rc<Font>>,
}

impl core::ops::Deref for Label {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl Label {
    pub fn new(form: &Rc<Form>) -> Rc<Self> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate("STATIC");
            let ex_style = WINDOW_EX_STYLE(0);

            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR::from_raw(class_name_wstr.as_ptr()),
                PCWSTR::from_raw(window_name.as_ptr()),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                0,
                0,
                parent_window,
                HMENU(0),       // hmenu,
                get_instance(), // hinstance,
                None,
            );

            if hwnd.0 == 0 {
                panic!("failed to create control window");
            }

            Rc::new(Label {
                control: ControlState::new(form, hwnd),
            })
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }
}
