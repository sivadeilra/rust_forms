use super::*;

pub struct App {
    state: Rc<AppState>,
}

assert_not_impl_any!(App: Send, Sync);

pub(crate) struct AppState {
    controls: HashMap<HWND, Control>,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: Rc::new(AppState {
                controls: HashMap::new(),
            }),
        }
    }
}

pub fn post_quit_message(exit_code: i32) {
    unsafe { PostQuitMessage(exit_code) }
}

pub fn event_loop() {
    unsafe {
        loop {
            let mut msg: MSG = zeroed();
            let ret = GetMessageW(&mut msg, 0, 0, 0).0;
            if ret < 0 {
                debug!("event loop: GetMessageW returned {}, quitting", ret);
                break;
            }

            if msg.message == WM_QUIT {
                debug!("found WM_QUIT, quitting");
                break;
            }

            TranslateMessage(&mut msg);
            DispatchMessageW(&mut msg);
        }
    }
}
