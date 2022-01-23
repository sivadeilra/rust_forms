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
    pub fn new(parent: &Rc<Form>) -> Rc<TextBox> {
        Self::new_with_options(parent, Default::default())
    }

    pub fn new_with_options(parent: &Rc<Form>, options: TextBoxOptions) -> Rc<TextBox> {
        unsafe {
            let class_name: U16CString = U16CString::from_str_truncate("Edit");
            let ex_style: u32 = 0;

            let mut style = WS_CHILD | WS_VISIBLE | WS_BORDER;
            if options.readonly {
                style |= ES_READONLY as u32;
            }
            if options.multiline {
                style |= ES_MULTILINE as u32;
            }
            if options.vertical_scrollbar {
                style |= WS_VSCROLL as u32;
            }

            let handle = CreateWindowExW(
                ex_style,
                PWSTR(class_name.as_ptr() as *mut _),
                PWSTR(null_mut()), // text
                style,
                0,
                0,
                0,
                0,
                Some(parent.handle.get()),
                None,       // menu
                None,       // instance
                null_mut(), // form_alloc.as_mut() as *mut UnsafeCell<FormState> as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            let control = ControlState {
                form: Rc::downgrade(&parent),
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

    pub fn set_readonly(&self, value: bool) {
        unsafe {
            SendMessageW(self.handle(), EM_SETREADONLY, value as WPARAM, 0);
        }
    }
}

#[derive(Default, Clone)]
pub struct TextBoxOptions {
    pub multiline: bool,
    pub readonly: bool,
    pub vertical_scrollbar: bool,
}
