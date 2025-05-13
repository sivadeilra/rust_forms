use super::*;
use windows::core::w;

pub struct FormBuilder<'a> {
    args: Box<FormArgs<'a>>,
}

struct FormArgs<'a> {
    text: Option<&'a str>,
    size: Option<(i32, i32)>,
    parent: Option<&'a Form>,
    quit_on_close: Option<i32>,
    style: Option<Rc<Style>>,
}

impl<'a> Default for FormBuilder<'a> {
    fn default() -> Self {
        Self {
            args: Box::new(FormArgs {
                text: None,
                size: None,
                parent: None,
                quit_on_close: Some(0),
                style: None,
            }),
        }
    }
}

impl<'a> FormBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parent(&mut self, parent: &'a Form) -> &mut Self {
        self.args.parent = Some(parent);
        self
    }

    pub fn text(&mut self, text: &'a str) -> &mut Self {
        self.args.text = Some(text);
        self
    }

    pub fn size(&mut self, w: i32, h: i32) -> &mut Self {
        self.args.size = Some((w, h));
        self
    }

    pub fn quit_on_close(&mut self) -> &mut Self {
        self.args.quit_on_close = Some(0);
        self
    }

    pub fn quit_on_close_with(&mut self, exit_code: i32) -> &mut Self {
        self.args.quit_on_close = Some(exit_code);
        self
    }

    pub fn no_quit_on_close(&mut self) -> &mut Self {
        self.args.quit_on_close = None;
        self
    }

    pub fn style(&mut self, style: Rc<Style>) -> &mut Self {
        self.args.style = Some(style);
        self
    }

    pub fn build(&mut self) -> Rc<Form> {
        crate::init::init_common_controls();

        let style = if let Some(s) = self.args.style.take() {
            s
        } else {
            Rc::new(Style::default())
        };

        unsafe {
            let co_initialized = match CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
                Ok(()) => {
                    debug!("CoInitializeEx succeeded");
                    true
                }
                Err(_) => {
                    warn!("CoInitializeEx failed");
                    false
                }
            };

            let window_class_atom = register_class_lazy();
            let instance = get_instance();

            let ex_style = WINDOW_EX_STYLE(0);

            let window_name_wstr: U16CString;
            let mut window_name_pwstr = PCWSTR::null();
            if let Some(text) = self.args.text {
                window_name_wstr = U16CString::from_str(text).unwrap();
                window_name_pwstr = PCWSTR::from_raw(window_name_wstr.as_ptr());
            }

            let mut width = CW_USEDEFAULT;
            let mut height = CW_USEDEFAULT;

            if let Some((w, h)) = self.args.size {
                width = w;
                height = h;
            }

            let form_alloc: Rc<Form> = Rc::new(Form {
                co_initialized,
                stuck: StuckToThread::new(),
                control: Default::default(),
                handle: Cell::new(HWND(null_mut())),
                quit_on_close: self.args.quit_on_close,
                is_layout_valid: Cell::new(false),
                layout: RefCell::new(None),
                layout_min_size: Cell::new((0, 0)),
                background_brush: Default::default(),
                background_color: Cell::new(ColorRef::from_sys_color(SysColor::Window)),
                status_bar: Cell::new(None),
                command_handler: Default::default(),
                notify_handler: Default::default(),
                tab_controls: Default::default(),
                style,
            });

            let form_alloc_ptr: *const Form = &*form_alloc;

            let parent_window_handle: Option<HWND> = if let Some(parent) = self.args.parent {
                Some(parent.handle())
            } else {
                None
            };

            let handle = match CreateWindowExW(
                ex_style,
                PCWSTR::from_raw(window_class_atom as usize as *const u16),
                window_name_pwstr,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                parent_window_handle,
                None,
                Some(instance),
                Some(form_alloc_ptr as *const c_void as *mut c_void),
            ) {
                Ok(h) => h,
                Err(e) => {
                    panic!("Failed to create window: {e:?}");
                }
            };

            // let _ = SetWindowTheme(handle, w!("EXPLORER"), PCWSTR::null());
            // let _ = SetWindowTheme(handle, w!("Window"), PCWSTR::null());

            let button_string = U16CString::from_str_truncate("BUTTON");
            let htheme = OpenThemeData(Some(handle), PCWSTR::from_raw(button_string.as_ptr()));
            if htheme.0 != 0 {
                debug!("ooo, got theme data");

                const BP_CHECKBOX: i32 = 3;
                const CBS_CHECKEDNORMAL: i32 = 5;

                let part = BP_CHECKBOX;
                if let Ok(color) =
                    GetThemeColor(htheme, part, CBS_CHECKEDNORMAL, THEME_PROPERTY_SYMBOL_ID(0))
                {
                    debug!("part {}, got theme color: 0x{:x}", part, color.0);
                } else {
                    warn!("part {}, failed to get theme color", part);
                }

                // dbg!(GetThemeSysColor(htheme, COLOR_MENUTEXT as i32));
            } else {
                warn!("failed to open theme data for window");
            }

            let htheme = GetWindowTheme(handle);

            let mut logfont: LOGFONTW = zeroed();
            match GetThemeSysFont(Some(htheme), TMT_STATUSFONT, &mut logfont) {
                Ok(f) => {
                    debug!(
                        "GetThemeFont succeeded: {}",
                        U16CString::from_ptr_str(logfont.lfFaceName.as_ptr()).to_string_lossy()
                    );
                    match Font::from_logfont(&logfont) {
                        Ok(f) => {
                            // form_alloc.default_static_font.set(Some(Rc::new(f)));
                        }
                        Err(_e) => {}
                    }
                    // Font::new(font_family, height)
                }
                Err(e) => {
                    warn!("GetThemeFont: {}", e);
                }
            }

            // Store the window handle, now that we know it, in the FormState.
            form_alloc.control.set(ControlState::new(handle)).unwrap();
            form_alloc.handle.set(handle);

            if let Ok(br) = Brush::from_sys_color(SysColor::Window) {
                form_alloc.background_brush.set(Some(br));
            }

            _ = SendMessageW(handle, WM_THEMECHANGED, None, None);

            /*
            match Font::new("Arial", 10) {
                Ok(font) => {
                    debug!("setting font");
                    form_alloc.default_static_font.set(Some(font));
                }
                Err(_e) => {}
            }
            */

            form_alloc
        }
    }
}
