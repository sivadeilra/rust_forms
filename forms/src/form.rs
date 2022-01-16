use super::*;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use log::debug;
use std::sync::Once;

pub struct Form {
    pub(crate) handle: WindowHandle,
    state: Box<UnsafeCell<FormState>>,
}

impl Form {
    pub fn builder<'a>() -> FormBuilder<'a> {
        FormBuilder::default()
    }

    pub fn show_window(&self) {
        unsafe {
            winuser::ShowWindow(self.handle.handle(), winuser::SW_SHOW);
        }
    }
}

pub struct FormBuilder<'a> {
    text: Option<&'a str>,
    size: Option<(i32, i32)>,
    quit_on_close: Option<i32>,
}

impl<'a> Default for FormBuilder<'a> {
    fn default() -> Self {
        Self {
            text: None,
            size: None,
            quit_on_close: Some(0),
        }
    }
}

impl<'a> FormBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(&mut self, text: &'a str) -> &mut Self {
        self.text = Some(text);
        self
    }

    pub fn size(&mut self, w: i32, h: i32) -> &mut Self {
        self.size = Some((w, h));
        self
    }

    pub fn quit_on_close(&mut self) -> &mut Self {
        self.quit_on_close = Some(0);
        self
    }

    pub fn quit_on_close_with(&mut self, exit_code: i32) -> &mut Self {
        self.quit_on_close = Some(exit_code);
        self
    }

    pub fn no_quit_on_close(&mut self) -> &mut Self {
        self.quit_on_close = None;
        self
    }

    pub fn build(&self) -> Form {
        unsafe {
            let window_class_atom = register_class_lazy();
            let instance = get_instance();

            let ex_style: u32 = 0;

            let mut window_name_wstr: U16CString;
            let mut window_name_pwstr: PWSTR = PWSTR(null_mut());
            if let Some(text) = self.text {
                window_name_wstr = U16CString::from_str(text).unwrap();
                window_name_pwstr = PWSTR(window_name_wstr.as_mut_ptr());
            }

            let mut width = CW_USEDEFAULT;
            let mut height = CW_USEDEFAULT;

            if let Some((w, h)) = self.size {
                width = w;
                height = h;
            }

            let form_alloc: Box<UnsafeCell<FormState>> = Box::new(UnsafeCell::new(FormState {
                quit_on_close: self.quit_on_close,
            }));

            let form_alloc_ptr: *mut FormState = form_alloc.get();

            let handle = winuser::CreateWindowExW(
                ex_style,
                PWSTR(window_class_atom as usize as *mut u16),
                window_name_pwstr,
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                None,
                None,
                instance,
                form_alloc_ptr as *mut c_void,
            );

            if handle == 0 {
                panic!("Failed to create window");
            }

            debug!(
                "created form window, hwnd {:8x}, state at {:?}",
                handle, form_alloc_ptr
            );

            Form {
                handle: WindowHandle::from_handle(handle),
                state: form_alloc,
            }
        }
    }
}

type ATOM = u16;
// use windows::Win32::UI::WindowsAndMessaging::ATOM;

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut FORM_CLASS_ATOM: ATOM = 0;

const FORM_CLASS_NAME: &str = "RustForms_Form";

fn get_instance() -> HINSTANCE {
    unsafe {
        let instance = windows::Win32::System::LibraryLoader::GetModuleHandleA(None);
        debug_assert!(instance != 0);
        instance
    }
}

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(FORM_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PWSTR(class_name_wstr.as_mut_ptr());
        class_ex.style = winuser::CS_HREDRAW | winuser::CS_VREDRAW;
        class_ex.hbrBackground = (winuser::COLOR_WINDOW + 1) as _;
        class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = winuser::LoadCursorW(0isize, winuser::IDC_ARROW);
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = winuser::RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        FORM_CLASS_ATOM = atom;
    });

    unsafe { FORM_CLASS_ATOM }
}

extern "system" fn form_wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {

        match message {
            winuser::WM_CREATE => {
                let create_struct: *mut winuser::CREATESTRUCTW =
                    lparam as *mut winuser::CREATESTRUCTW;
                assert!(!create_struct.is_null());

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                debug!(
                    "WM_CREATE, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                winuser::SetWindowLongPtrW(window, 0, create_params as isize);
                return 1;
            }

            _ => {}
        }

        let state_ptr: isize = winuser::GetWindowLongPtrW(window, 0);
        if state_ptr == 0 {
            debug!("form_wndproc: lparam is null, msg {:04x}", message);
            return winuser::DefWindowProcW(window, message, wparam, lparam);
        }

        let state: &FormState = &*(state_ptr as *const FormState);

        match message {
            /*
            WM_PAINT => {
                println!("WM_PAINT");
                // ValidateRect(window, std::ptr::null());
                0
            }
            */

            winuser::WM_CLOSE => {
                if let Some(exit_code) = state.quit_on_close {
                    debug!("WM_CLOSE: posting quit message");
                    post_quit_message(exit_code);
                } else {
                    debug!("WM_CLOSE: not posting quit message");
                }
            }

            winuser::WM_DESTROY => {
                println!("WM_DESTROY");
                return 0;
            }
            _ => {
                // allow default to run
            }
        }

        winuser::DefWindowProcW(window, message, wparam, lparam)
    }
}

/// This is the data that is pointed-to by the window user data field.
struct FormState {
    quit_on_close: Option<i32>,
}
