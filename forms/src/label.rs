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
    pub fn new(form: &Form, rect: Option<&Rect>) -> Rc<Self> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate("STATIC");
            let ex_style = 0;

            let rect = rect_or_default(rect);

            let hwnd = CreateWindowExW(
                ex_style,
                PWSTR(class_name_wstr.as_ptr() as *mut _),
                PWSTR(window_name.as_ptr() as *mut _),
                WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                parent_window,
                0 as HMENU,     // hmenu,
                get_instance(), // hinstance,
                null_mut(),
            );

            if hwnd == 0 {
                panic!("failed to create control window");
            }

            let this = Rc::new(Label {
                control: ControlState {
                    handle: hwnd,
                    layout: RefCell::new(ControlLayout::default()),
                    form: Rc::downgrade(&form.state),
                },
                // font: Default::default(),
            });

            /*
            if let Some(font) = form.get_default_button_font() {
                this.set_font(font);
            }
            */

            this
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }
}
