use super::*;

pub struct TextBox {
    control: Control,
}

impl core::ops::Deref for TextBox {
    type Target = Control;
    fn deref(&self) -> &Control {
        &self.control
    }
}

impl TextBox {
    pub fn new(parent: &Form, rect: &Rect) -> TextBox {
        unsafe {
            let class_name: U16CString = U16CString::from_str_truncate("Edit");

            let ex_style: u32 = 0;

            let handle = winuser::CreateWindowExW(
                ex_style,
                PWSTR(class_name.as_ptr() as *mut _),
                PWSTR(null_mut()), // text
                winuser::WS_CHILD | WS_VISIBLE | winuser::WS_BORDER,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                Some(parent.handle.handle()),
                None,       // menu
                None,       // instance
                null_mut(), // form_alloc.as_mut() as *mut UnsafeCell<FormState> as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            TextBox {
                control: Control::from_handle(WindowHandle::from_handle(handle)),
            }
        }
    }

    pub fn set_text(&self, s: &str) {
        unsafe {
            let s_wstr: U16CString = U16CString::from_str(s).unwrap();
            SendMessageW(
                self.control.handle(),
                winuser::WM_SETTEXT,
                0,
                s_wstr.as_ptr() as LPARAM,
            );
        }
    }
}
