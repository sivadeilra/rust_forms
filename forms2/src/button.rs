use super::*;

pub struct Button {
    state: Rc<ButtonState>,
}

pub(crate) struct ButtonState {
    control: ControlState,
    on_click: Cell<Option<Rc<EventHandler<()>>>>,
}

impl Button {
    pub fn new(form: &Form) -> Self {
        unsafe {
            let parent_window = form.handle();

            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate("BUTTON");

            let x = 0;
            let y = 450;
            let nwidth = 300;
            let nheight = 50;

            let ex_style = 0;

            let windowHandle = CreateWindowExW(
                ex_style,
                PWSTR(class_name_wstr.as_ptr() as *mut _),
                PWSTR(window_name.as_ptr() as *mut _),
                WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER,
                x,
                y,
                nwidth,
                nheight,
                parent_window,
                0 as HMENU,     // hmenu,
                get_instance(), // hinstance,
                null_mut(),
            );

            if windowHandle == 0 {
                panic!("failed to create button window");
            }

            debug!("created list view window 0x{:x}", windowHandle);

            let control = ControlState {
                controls: Vec::new(),
                handle: windowHandle,
                layout: ControlLayout::default(),
                form: Rc::downgrade(&form.state),
            };

            form.state.invalidate_layout();

            let mut state = Rc::new(ButtonState {
                control,
                on_click: Cell::new(None),
            });

            {
                let mut event_handlers = form.state.event_handlers.borrow_mut();
                let handler_rc: Rc<ButtonState> = Rc::clone(&state);
                event_handlers.insert(windowHandle, handler_rc);
            }

            let mut controls = form.state.controls.borrow_mut();
            Self { state }
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.state.control.handle, text);
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
