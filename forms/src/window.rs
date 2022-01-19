// use super::*;

pub struct WindowHandle {
    handle: isize,
}

impl WindowHandle {
    pub fn handle(&self) -> isize {
        self.handle
    }

    /// This is unsafe because it enables the drop handler to run, which
    /// calls DestroyWindow.
    pub unsafe fn from_handle(handle: isize) -> Self {
        Self { handle }
    }
}

impl Drop for WindowHandle {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.handle);
        }
    }
}
