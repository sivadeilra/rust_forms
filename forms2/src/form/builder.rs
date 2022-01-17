use super::*;

pub struct FormBuilder<'a> {
    text: Option<&'a str>,
    size: Option<(i32, i32)>,
    quit_on_close: Option<i32>,
}

impl<'a> Default for FormBuilder<'a> {
    fn default() -> Self {
        Self {
            text: None,
            size: None,
            quit_on_close: Some(0),
        }
    }
}

impl<'a> FormBuilder<'a> {
    pub fn new() -> Self {
        init_common_controls();

        Self::default()
    }

    pub fn text(&mut self, text: &'a str) -> &mut Self {
        self.text = Some(text);
        self
    }

    pub fn size(&mut self, w: i32, h: i32) -> &mut Self {
        self.size = Some((w, h));
        self
    }

    pub fn quit_on_close(&mut self) -> &mut Self {
        self.quit_on_close = Some(0);
        self
    }

    pub fn quit_on_close_with(&mut self, exit_code: i32) -> &mut Self {
        self.quit_on_close = Some(exit_code);
        self
    }

    pub fn no_quit_on_close(&mut self) -> &mut Self {
        self.quit_on_close = None;
        self
    }

    pub fn build(&self) -> Form {
        unsafe {
            let window_class_atom = register_class_lazy();
            let instance = get_instance();

            let ex_style: u32 = 0;

            let mut window_name_wstr: U16CString;
            let mut window_name_pwstr: PWSTR = PWSTR(null_mut());
            if let Some(text) = self.text {
                window_name_wstr = U16CString::from_str(text).unwrap();
                window_name_pwstr = PWSTR(window_name_wstr.as_mut_ptr());
            }

            let mut width = CW_USEDEFAULT;
            let mut height = CW_USEDEFAULT;

            if let Some((w, h)) = self.size {
                width = w;
                height = h;
            }

            let form_alloc: Rc<FormState> = Rc::new(FormState {
                handle: Cell::new(0),
                quit_on_close: self.quit_on_close,
                controls: RefCell::new(HashMap::new()),
                is_visible: Cell::new(false),
                is_layout_valid: Cell::new(false),
                notify_handlers: RefCell::new(HashMap::new()),
                event_handlers: RefCell::new(HashMap::new()),
                default_edit_font: Default::default(),
                default_button_font: Default::default(),
                receivers: Default::default(),
            });

            let form_alloc_ptr: *const FormState = &*form_alloc;

            let handle = CreateWindowExW(
                ex_style,
                PWSTR(window_class_atom as usize as *mut u16),
                window_name_pwstr,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                None,
                None,
                instance,
                form_alloc_ptr as *const c_void as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            debug!(
                "created form window, hwnd {:8x}, state at {:?}",
                handle, form_alloc_ptr
            );

            // Store the window handle, now that we know it, in the FormState.
            form_alloc.handle.set(handle);

            Form { state: form_alloc }
        }
    }
}
