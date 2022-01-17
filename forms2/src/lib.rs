#![allow(warnings)]

mod app;
mod button;
mod control;
mod font;
mod form;
mod layout;
mod list_view;
mod text_box;
mod error;

pub use error::{Error, Result};
pub use app::*;
pub use button::*;
pub use control::*;
pub use font::*;
pub use form::*;
pub use layout::*;
pub use list_view::*;
pub use text_box::*;
pub use windows::Win32::Foundation::RECTL as Rect;

use core::any::Any;
use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use log::{debug, error, trace};
use static_assertions::assert_not_impl_any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use widestring::U16CStr as WCStr;
use widestring::U16CString as WCString;
use widestring::U16Str as WStr;
use widestring::{U16CStr, U16CString};
use windows::Win32::Foundation::{BOOL, HWND, PWSTR, *};
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Size(i32, i32);

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Point(i32, i32);

/// Wraps a function that can handle an event of a given type.
pub struct EventHandler<E> {
    pub(crate) handler: Box<dyn Fn(E)>,
}

impl<E> EventHandler<E> {
    pub fn new<H>(handler: H) -> Self
    where
        H: Fn(E) + 'static,
    {
        Self {
            handler: Box::new(handler),
        }
    }
}

pub struct ControlHost {}

pub(crate) fn set_window_text(hwnd: HWND, text: &str) {
    unsafe {
        let ws = WCString::from_str_truncate(text);
        SendMessageW(hwnd, WM_SETTEXT, 0, ws.as_ptr() as LPARAM);
    }
}

pub(crate) fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let len_lresult = SendMessageW(hwnd, WM_GETTEXTLENGTH, 0, 0);
        assert!(len_lresult >= 0, "WM_GETTEXTLENGTH returned bogus value");
        let len = len_lresult as usize;
        let capacity = len + 1;
        let mut buffer: Vec<u16> = Vec::with_capacity(capacity);
        let len_copied = SendMessageW(
            hwnd,
            WM_GETTEXT,
            capacity as WPARAM,
            buffer.as_mut_ptr() as LPARAM,
        );
        assert!(len_copied >= 0);
        assert!((len_copied as usize) <= len);
        buffer.set_len(len_copied as usize);
        widestring::U16Str::from_slice(buffer.as_slice()).to_string_lossy()
    }
}

fn init_common_controls() {
    INIT_COMMON_CONTROLS.call_once(|| unsafe {
        let mut icc: INITCOMMONCONTROLSEX = zeroed();
        icc.dwSize = size_of::<INITCOMMONCONTROLSEX>() as u32;
        icc.dwICC = ICC_LISTVIEW_CLASSES;
        InitCommonControlsEx(&icc);
        SetThemeAppProperties(STAP_ALLOW_NONCLIENT | STAP_ALLOW_CONTROLS | STAP_ALLOW_WEBCONTENT)
    });
}

const STAP_ALLOW_NONCLIENT: u32 = 1 << 0;
const STAP_ALLOW_CONTROLS: u32 = 1 << 1;
const STAP_ALLOW_WEBCONTENT: u32 = 1 << 2;

use std::sync::Once;

static INIT_COMMON_CONTROLS: Once = Once::new();

pub(crate) const WM_NOTIFY: u32 = 0x004E;

fn clone_cell_opt_rc<T>(rc: &Cell<Option<Rc<T>>>) -> Option<Rc<T>> {
    let value = rc.take();
    let result = value.clone();
    rc.set(value);
    result
}
