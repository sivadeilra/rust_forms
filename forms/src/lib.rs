pub mod asyncui;
pub mod control;
mod events;
mod form;
mod layout;
pub mod list_view;
mod text_box;
mod tree;
pub mod window;

pub use control::*;
pub use events::*;
pub use form::*;
pub use layout::*;
pub use list_view::*;
pub use text_box::*;
pub use window::*;

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use widestring::{U16CStr, U16CString};
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::WindowsAndMessaging as winuser;
// use winuser::{HWND, LPARAM, WPARAM, LRESULT};
use windows::Win32::Foundation::*;
use windows::Win32::Foundation::{HINSTANCE, PWSTR};
use winuser::{CW_USEDEFAULT, WNDCLASSEXW, WS_OVERLAPPEDWINDOW, WS_VISIBLE};

use winuser::SendMessageW;

pub use windows::Win32::Foundation::RECTL as Rect;

pub fn init_common_controls() {
    unsafe {
        let mut icc: INITCOMMONCONTROLSEX = zeroed();
        icc.dwSize = size_of::<INITCOMMONCONTROLSEX>() as u32;
        icc.dwICC = ICC_LISTVIEW_CLASSES;
        InitCommonControlsEx(&icc);
        SetThemeAppProperties(STAP_ALLOW_NONCLIENT | STAP_ALLOW_CONTROLS | STAP_ALLOW_WEBCONTENT)
    }
}

const STAP_ALLOW_NONCLIENT: u32 = 1 << 0;
const STAP_ALLOW_CONTROLS: u32 = 1 << 1;
const STAP_ALLOW_WEBCONTENT: u32 = 1 << 2;
