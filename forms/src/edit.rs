use super::*;

pub struct Edit {
    control: ControlState,
    font: Cell<Option<Rc<Font>>>,
}

#[derive(Default, Clone)]
pub struct EditOptions {
    pub multiline: bool,
    pub readonly: bool,
    pub vertical_scrollbar: bool,
    pub no_hide_selection: bool,
    pub want_return: bool,
}

impl core::ops::Deref for Edit {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl Edit {
    pub fn new(parent: &Rc<Form>) -> Rc<Edit> {
        Self::new_with_options(parent, Default::default())
    }

    pub fn new_with_options(form: &Rc<Form>, options: EditOptions) -> Rc<Edit> {
        unsafe {
            let class_name: U16CString = U16CString::from_str_truncate("Edit");
            let ex_style: u32 = 0;

            let mut style = WS_CHILD | WS_VISIBLE | WS_BORDER;
            if options.readonly {
                style |= ES_READONLY as u32;
            }
            if options.multiline {
                style |= ES_MULTILINE as u32 | ES_AUTOVSCROLL as u32;
            } else {
                style |= ES_AUTOHSCROLL as u32;
            }
            if options.no_hide_selection {
                style |= ES_NOHIDESEL as u32;
            }
            if options.vertical_scrollbar {
                style |= WS_VSCROLL as u32;
            }
            if options.want_return {
                style |= ES_WANTRETURN as u32;
            }

            let handle = CreateWindowExW(
                ex_style,
                PWSTR(class_name.as_ptr() as *mut _),
                PWSTR(null_mut()),
                style,
                0,
                0,
                0,
                0,
                Some(form.handle()),
                None,       // menu
                None,       // instance
                null_mut(), // form_alloc.as_mut() as *mut UnsafeCell<FormState> as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            let control = ControlState::new(form, handle);

            let this = Rc::new(Edit {
                control,
                font: Default::default(),
            });

            if let Some(font) = form.get_default_edit_font() {
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

    pub fn enable_autocomplete(&self) {
        unsafe {
            use Win32::UI::Shell::*;
            let r = SHAutoComplete(
                self.handle(),
                SHACF_AUTOAPPEND_FORCE_OFF | SHACF_FILESYS_ONLY,
            );
            debug!("SHAutoComplete: {:?}", r);
        }
    }
}