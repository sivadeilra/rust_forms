use super::*;
use log::debug;

pub fn event_loop() {
    unsafe {
        loop {
            let mut msg: winuser::MSG = zeroed();
            let ret = winuser::GetMessageW(&mut msg, 0, 0, 0).0;
            if ret < 0 {
                debug!("event loop: GetMessageW returned {}, quitting", ret);
                break;
            }

            if msg.message == winuser::WM_QUIT {
                debug!("found WM_QUIT, quitting");
                break;
            }

            winuser::TranslateMessage(&mut msg);
            winuser::DispatchMessageW(&mut msg);
        }
    }
}

pub fn post_quit_message(exit_code: i32) {
    unsafe { winuser::PostQuitMessage(exit_code) }
}
