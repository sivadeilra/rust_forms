use super::*;

#[derive(Clone)]
pub struct Label {
    control: Rc<ControlState>,
}

impl core::ops::Deref for Label {
    type Target = Rc<ControlState>;
    fn deref(&self) -> &Rc<ControlState> {
        &self.control
    }
}

impl Label {
    pub fn new(form: &Form, text: &str) -> Self {
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
                Some(parent_window),
                None,                 // hmenu,
                Some(get_instance()), // hinstance,
                None,
            )
            .unwrap();

            let control = ControlState::new(hwnd);
            control.set_font(&form.rc.style.static_font);
            control.set_text(text);

            Label { control }
        }
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
