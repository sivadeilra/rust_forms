use std::sync::Once;

use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

use super::*;

pub fn post_quit_message(exit_code: i32) {
    unsafe { PostQuitMessage(exit_code) }
}

pub fn event_loop() {
    unsafe {
        loop {
            let mut msg: MSG = zeroed();
            let ret = GetMessageW(&mut msg, None, 0, 0).0;
            if ret < 0 {
                debug!("event loop: GetMessageW returned {}, quitting", ret);
                break;
            }

            if msg.message == WM_QUIT {
                debug!("found WM_QUIT, quitting");
                break;
            }

            _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// App is the first object any Forms app creates. It contains shared resource and state.
#[derive(Clone)]
pub struct App {
    pub(crate) state: Rc<AppState>,
}

static COINIT_CALLED: Once = Once::new();

fn coinit() {
    COINIT_CALLED.call_once(|| unsafe {
        match CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok() {
            Ok(()) => {
                debug!("CoInitializeEx succeeded");
            }
            Err(_) => {
                warn!("CoInitializeEx failed");
            }
        };
    });

    /*
            if self.co_initialized {
                CoUninitialize();
            }
    */
}

impl App {
    /// Creates a new app. This is the first step in any Forms app.
    pub fn new() -> App {
        coinit();
        Self {
            state: Rc::new(AppState {
                button_background_brush: unsafe { CreateSolidBrush(COLORREF(0xe0_ff_00_ff)) },
            }),
        }
    }

    pub fn form_builder(&self) -> FormBuilder {
        FormBuilder {
            app: self.clone(),
            title: None,
            size: None,
            parent: None,
            quit_on_close: Some(0),
            style: None,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) struct AppState {
    pub(crate) button_background_brush: HBRUSH,
}
