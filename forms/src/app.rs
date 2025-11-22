use std::collections::VecDeque;
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

#[cfg(feature = "simple-tracing")]
mod simple_tracing {
    use std::sync::Once;

    static TRACING_INIT: Once = Once::new();

    pub(crate) fn init() {
        TRACING_INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
        });
    }
}

impl App {
    /// Creates a new app. This is the first step in any Forms app.
    pub fn new() -> App {
        coinit();

        #[cfg(feature = "simple-tracing")]
        {
            simple_tracing::init();
        }

        Self {
            state: Rc::new(AppState {
                button_background_brush: unsafe { CreateSolidBrush(COLORREF(0xe0_ff_00_ff)) },
                event_queue: RefCell::new(VecDeque::new()),
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

    /// This is a queue of events that we will report to the app. The events are written to this
    /// so that they can be processed safely _after_ we have exited the DispatchMessageW call.
    pub(crate) event_queue: RefCell<VecDeque<AppEvent>>,
}

impl App {
    pub fn wait_event(&self) -> Option<AppEvent> {
        unsafe {
            loop {
                // See if there is already an app event.
                {
                    let mut event_queue = self.state.event_queue.borrow_mut();
                    if let Some(event) = event_queue.pop_front() {
                        return Some(event);
                    }
                }

                let mut msg: MSG = zeroed();
                let ret = GetMessageW(&mut msg, None, 0, 0).0;
                if ret < 0 {
                    debug!("event loop: GetMessageW returned {}, quitting", ret);
                    return None;
                }

                debug!("wm 0x{:04x}", msg.message);

                if msg.message == WM_QUIT {
                    debug!("found WM_QUIT, quitting");
                    return None;
                }

                // if IsDialogMessageW(self.handle(), &msg).into() {
                //     continue;
                // }

                _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);

                // Continue the loop. If DispatchMessageW contributed an event to event_queue,
                // then we will dequeue it and return. Otherwise, we'll wait for another UI message.
            }
        }
    }
}

impl AppState {
    pub(crate) fn push_event(&self, event: AppEvent) {
        let mut event_queue = self.event_queue.borrow_mut();
        event_queue.push_back(event);
    }
}

#[derive(Debug)]
pub enum AppEvent {
    Quit,

    Notify { control: ControlId, notify: Notify },
}

#[derive(Debug)]
pub enum Notify {
    ButtonClicked,
}
