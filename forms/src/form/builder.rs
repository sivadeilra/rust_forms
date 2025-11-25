use super::*;
use windows::core::w;

pub struct FormBuilder {
    pub(crate) app: App,
    pub(crate) title: String,
    pub(crate) size: Option<(i32, i32)>,
    pub(crate) parent: Option<Form>,
    pub(crate) quit_on_close: Option<i32>,
    pub(crate) style: Option<Rc<Style>>,

    pub(crate) mdi_parent: Option<Form>,
    pub(crate) mdi_mode: MdiMode,
}

impl FormBuilder {
    // pub fn parent(&mut self, parent: &Form) -> &mut Self {
    //     self.args.parent = Some(parent);
    //     self
    // }

    pub fn mdi_parent(&mut self, parent: &Form) -> &mut Self {
        assert!(parent.rc.mdi_mode == MdiMode::Frame);
        assert!(self.mdi_mode == MdiMode::None);
        self.mdi_mode = MdiMode::Child;
        self.mdi_parent = Some(parent.clone());
        self.no_quit_on_close();
        self
    }

    pub fn mdi_frame(&mut self) {
        assert!(self.mdi_mode == MdiMode::None);
        self.mdi_mode = MdiMode::Frame;
    }

    pub fn title(&mut self, text: &str) -> &mut Self {
        self.title = text.to_string();
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

    pub fn style(&mut self, style: Rc<Style>) -> &mut Self {
        self.style = Some(style);
        self
    }

    pub fn build(self) -> Form {
        crate::init::init_common_controls();

        let style = if let Some(s) = self.style {
            s
        } else {
            Rc::new(Style::default())
        };

        unsafe {
            let instance = get_instance();
            let ex_style = WINDOW_EX_STYLE(0);
            let window_name_wstr = U16CString::from_str_truncate(&self.title);
            let window_name_pwstr = PCWSTR::from_raw(window_name_wstr.as_ptr());

            let mut width = CW_USEDEFAULT;
            let mut height = CW_USEDEFAULT;
            if let Some((w, h)) = self.size {
                width = w;
                height = h;
            }

            let parent_window_handle: Option<HWND> = if let Some(parent) = &self.parent {
                Some(parent.handle())
            } else {
                None
            };

            let create_params: u32 = 0;
            let create_params_lparam: *mut c_void = null_mut();

            let handle: HWND;

            match self.mdi_mode {
                MdiMode::None | MdiMode::Frame => {
                    let window_class_atom = wndproc::register_class_lazy();
                    handle = match CreateWindowExW(
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
                        Some(create_params_lparam),
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            panic!("Failed to create window: {e:?}");
                        }
                    };
                }

                MdiMode::Child => {
                    let child_class_atom = wndproc::register_mdi_child_lazy();
                    let mdi_parent_form = self.mdi_parent.as_ref().unwrap();

                    let mdi_create = MDICREATESTRUCTW {
                        szClass: PCWSTR::from_raw(child_class_atom as usize as *const u16),
                        szTitle: window_name_pwstr,
                        hOwner: HANDLE(instance.0),
                        x: CW_USEDEFAULT,
                        y: CW_USEDEFAULT,
                        cx: width,
                        cy: height,
                        style: WINDOW_STYLE(MDIS_ALLCHILDSTYLES), // WS_MINIMIZE | WS_MAXIMIZE | WS_VISIBLE | WS_OVERLAPPED,
                        lParam: LPARAM(create_params_lparam as isize), // lparam
                    };

                    let Some(ref parent_mdi_client) = mdi_parent_form.rc.mdi_client else {
                        panic!("Parent form must be an MDI Frame");
                    };
                    let mdi_client_hwnd = parent_mdi_client.handle();
                    assert!(!mdi_client_hwnd.is_invalid());

                    about_to_create_window();

                    let mdi_child_lresult = SendMessageW(
                        mdi_client_hwnd,
                        WM_MDICREATE,
                        None,
                        Some(LPARAM(&mdi_create as *const _ as isize)),
                    );
                    if mdi_child_lresult.0 == 0 {
                        panic!("failed to create MDI child window");
                    }

                    handle = HWND(mdi_child_lresult.0 as *mut c_void);
                }
            }

            // TODO: pass parent form handle?
            let control = ControlState::new(handle, None);

            // If we are creating an MDI Frame, then create the MDI Client window. The MDI Client
            // is provided by the system; we do not provide a wndproc for it. It handles positioning
            // MDI child windows, their menus, etc. There is exactly one MDI client window for each
            // MDI frame. The MDI client window manages the CHILD windows.
            // (frame != client != child)

            let mdi_client: Option<MdiClient> = if self.mdi_mode == MdiMode::Frame {
                let ccs = CLIENTCREATESTRUCT {
                    hWindowMenu: HANDLE(null_mut()),
                    idFirstChild: 100,
                };

                let mdi_client_hwnd: HWND = match CreateWindowExW(
                    WINDOW_EX_STYLE(0), // ex_style
                    w!("MDICLIENT"),
                    window_name_pwstr,
                    WS_CHILD | WS_CLIPCHILDREN | WS_VSCROLL | WS_HSCROLL | WS_VISIBLE,
                    CW_USEDEFAULT, // x
                    CW_USEDEFAULT, // y
                    width,         // width
                    height,        // height
                    Some(handle),  // parent window handle (the frame)
                    None,          // hmenu
                    Some(instance),
                    Some(&ccs as *const _ as *const _), // lparam
                ) {
                    Ok(h) => h,
                    Err(_) => panic!("Failed to create MDI client window"),
                };
                debug!("successfully created MDICLIENT");
                Some(MdiClient {
                    control: ControlState::new(mdi_client_hwnd, Some(control.clone())),
                })
            } else {
                None
            };

            let layout: Option<Layout> = if let Some(ref mdi_client) = mdi_client {
                Some(Layout::Control(mdi_client.control.clone()))
            } else {
                None
            };

            let background_sys_color = SysColor::Window;

            let form_alloc: Rc<FormState> = Rc::new(FormState {
                app: self.app.clone(),
                stuck: StuckToThread::new(),
                control,
                quit_on_close: self.quit_on_close,
                is_layout_valid: Cell::new(false),
                layout: RefCell::new(layout),
                layout_min_size: Cell::new((0, 0)),
                background_brush: Default::default(),
                background_color: Cell::new(ColorRef::from_sys_color(background_sys_color)),
                status_bar: Cell::new(None),
                command_handler: Default::default(),
                tab_controls: Default::default(),
                style,
                mdi_mode: self.mdi_mode,
                mdi_client,
            });

            let form_alloc_ptr: *const FormState = &*form_alloc;

            SetWindowLongPtrW(handle, WINDOW_LONG_PTR_INDEX(0), form_alloc_ptr as isize);

            if let Ok(br) = Brush::from_sys_color(background_sys_color) {
                _ = SetClassLongPtrW(handle, GCLP_HBRBACKGROUND, br.handle().0 as isize);
                *form_alloc.background_brush.borrow_mut() = Some(br);
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

            Form { rc: form_alloc }
        }
    }
}

#[inline(never)]
fn about_to_create_window() {
    debug!("about to create window");
}
