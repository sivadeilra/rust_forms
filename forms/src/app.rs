use super::*;

pub fn post_quit_message(exit_code: i32) {
    unsafe { PostQuitMessage(exit_code) }
}

pub fn event_loop() {
    unsafe {
        loop {
            let mut msg: MSG = zeroed();
            let ret = GetMessageW(&mut msg, HWND(0), 0, 0).0;
            if ret < 0 {
                debug!("event loop: GetMessageW returned {}, quitting", ret);
                break;
            }

            if msg.message == WM_QUIT {
                debug!("found WM_QUIT, quitting");
                break;
            }

            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
