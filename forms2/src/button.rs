use super::*;

pub struct Button {
    control: Control,
    state: Rc<ButtonState>,
}

impl core::ops::Deref for Button {
    type Target = Control;
    fn deref(&self) -> &Control {
        &self.control
    }
}

pub(crate) struct ButtonState {
    font: Cell<Option<Rc<Font>>>,
    on_click: Cell<Option<Rc<EventHandler<()>>>>,
}

impl Button {
    pub fn new(form: &Form, rect: &Rect) -> Self {
        unsafe {
            let parent_window = form.handle();

            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate("BUTTON");

            let ex_style = 0;

            let windowHandle = CreateWindowExW(
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

            if windowHandle == 0 {
                panic!("failed to create button window");
            }

            debug!("created list view window 0x{:x}", windowHandle);

            let control = Control {
                state: Rc::new(ControlState {
                    controls: Vec::new(),
                    handle: windowHandle,
                    layout: RefCell::new(ControlLayout::default()),
                    form: Rc::downgrade(&form.state),
                }),
            };

            form.state.invalidate_layout();

            let mut state = Rc::new(ButtonState {
                on_click: Cell::new(None),
                font: Default::default(),
            });

            {
                let mut event_handlers = form.state.event_handlers.borrow_mut();
                let handler_rc: Rc<ButtonState> = Rc::clone(&state);
                event_handlers.insert(windowHandle, handler_rc);
            }

            let mut controls = form.state.controls.borrow_mut();
            let this = Self { control, state };

            if let Some(font) = form.get_default_button_font() {
                this.set_font(font);
            }

            this
        }
    }

    pub fn set_font(&self, font: Rc<Font>) {
        trace!("edit: setting font");
        unsafe {
            SendMessageW(self.control.handle(), WM_SETFONT, font.hfont as WPARAM, 1);
            self.state.font.set(Some(font));
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }

    pub fn on_clicked(&self, handler: EventHandler<()>) {
        self.state.on_click.set(Some(Rc::new(handler)));
    }
}

impl MessageHandlerTrait for ButtonState {
    fn handle_message(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        0
    }

    fn wm_command(&self, control_id: u16, notify_code: u16) -> LRESULT {
        match notify_code as u32 {
            BN_CLICKED => {
                if let Some(on_click) = self.on_click.take() {
                    let on_click_clone = Rc::clone(&on_click);
                    self.on_click.set(Some(on_click)); // put it back first
                    (*on_click_clone.handler)(());
                }
                0
            }

            _ => {
                debug!(
                    "button - unrecognized notification code: 0x{:x}",
                    notify_code
                );
                0
            }
        }
    }
}
