use super::*;

pub struct TextBox {
    control: ControlState,
    font: Cell<Option<Rc<Font>>>,
}

impl core::ops::Deref for TextBox {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl TextBox {
    pub fn new(parent: &Form, rect: Option<&Rect>) -> Rc<TextBox> {
        unsafe {
            let class_name: U16CString = U16CString::from_str_truncate("Edit");
            let ex_style: u32 = 0;
            let rect = rect_or_default(rect);

            let handle = CreateWindowExW(
                ex_style,
                PWSTR(class_name.as_ptr() as *mut _),
                PWSTR(null_mut()), // text
                WS_CHILD | WS_VISIBLE | WS_BORDER,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                Some(parent.state.handle.get()),
                None,       // menu
                None,       // instance
                null_mut(), // form_alloc.as_mut() as *mut UnsafeCell<FormState> as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            let control = ControlState {
                form: Rc::downgrade(&parent.state),
                handle,
                layout: Default::default(),
            };

            let this = Rc::new(TextBox {
                control,
                font: Default::default(),
            });

            if let Some(font) = parent.get_default_edit_font() {
                this.set_font(font);
            }

            this
        }
    }

    pub fn set_font(&self, font: Rc<Font>) {
        unsafe {
            SendMessageW(self.control.handle(), WM_SETFONT, font.hfont as WPARAM, 1);
            self.font.set(Some(font));
        }
    }

    pub fn set_text(&self, s: &str) {
        set_window_text(self.control.handle(), s);
    }

    pub fn get_text(&self) -> String {
        get_window_text(self.control.handle())
    }
}
