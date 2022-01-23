use super::*;

pub struct Button {
    control: ControlState,
    font: Cell<Option<Rc<Font>>>,
    on_click: Cell<Option<Rc<EventHandler<()>>>>,
}

impl core::ops::Deref for Button {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl Button {
    pub fn new(form: &Rc<Form>) -> Rc<Button> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate("BUTTON");
            let ex_style = 0;

            let hwnd = CreateWindowExW(
                ex_style,
                PWSTR(class_name_wstr.as_ptr() as *mut _),
                PWSTR(window_name.as_ptr() as *mut _),
                WS_CHILD | WS_VISIBLE,
                0,
                0,
                0,
                0,
                parent_window,
                0 as HMENU,     // hmenu,
                get_instance(), // hinstance,
                null_mut(),
            );

            if hwnd == 0 {
                panic!("failed to create button window");
            }

            let this = Rc::new(Button {
                control: ControlState {
                    handle: hwnd,
                    layout: RefCell::new(ControlLayout::default()),
                    form: Rc::downgrade(&form),
                },
                on_click: Cell::new(None),
                font: Default::default(),
            });

            {
                let mut event_handlers = form.event_handlers.borrow_mut();
                let handler_rc: Rc<Button> = Rc::clone(&this);
                event_handlers.insert(hwnd, handler_rc);
            }

            // let controls = form.state.controls.borrow_mut();

            if let Some(font) = form.get_default_button_font() {
                this.set_font(font);
            }

            this
        }
    }

    pub fn set_enabled(&self, value: bool) {
        unsafe {
            EnableWindow(self.control.handle, value);
        }
    }

    pub fn set_font(&self, font: Rc<Font>) {
        unsafe {
            SendMessageW(self.control.handle(), WM_SETFONT, font.hfont as WPARAM, 1);
            self.font.set(Some(font));
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }

    pub fn on_clicked(&self, handler: EventHandler<()>) {
        self.on_click.set(Some(Rc::new(handler)));
    }
}

impl MessageHandlerTrait for Button {
    fn handle_message(&self, _msg: u32, _wparam: WPARAM, _lparam: LPARAM) -> LRESULT {
        0
    }

    fn wm_command(&self, _control_id: u16, notify_code: u16) -> LRESULT {
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
