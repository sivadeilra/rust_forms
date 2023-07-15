#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::single_match)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_else_if)]

mod app;
mod brush;
mod button;
mod color;
mod command;
mod control;
mod edit;
mod error;
mod executor;
mod ffi;
pub mod file_dialog;
mod font;
mod form;
pub mod init;
mod label;
pub mod layout;
pub mod list_view;
mod menu;
mod messenger;
mod msg;
mod status_bar;
mod system_params;
mod tab;
pub mod tree_view;

pub use app::*;
pub use brush::{Brush, SysColor};
pub use button::*;
pub use color::*;
pub use command::*;
pub use control::*;
pub use edit::*;
pub use error::{Error, Result};
pub use executor::*;
pub use font::*;
pub use form::*;
pub use label::Label;
pub use layout::grid::*;
pub use layout::*;
pub use list_view::{ListView, Mode};
pub use menu::*;
pub use messenger::{Messenger, Sender};
pub use msg::*;
pub use status_bar::*;
pub use tab::*;
pub use tree_view::{TreeNode, TreeView, TreeViewOptions};
pub use windows::Win32::Foundation::RECTL as Rect;

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::mem::{size_of, size_of_val, zeroed};
use core::ptr::{null, null_mut};
use ffi::*;
use log::{debug, error, trace, warn};
use static_assertions::assert_not_impl_any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use widestring::U16CStr as WCStr;
use widestring::U16CString as WCString;
use widestring::U16CString;
use widestring::U16Str as WStr;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{HDWP, *};

// TODO: We currently leak these types. Fix that.
pub use windows::Win32::Foundation::POINT;

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
        SendMessageW(hwnd, WM_SETTEXT, WPARAM(0), LPARAM(ws.as_ptr() as isize));
    }
}

pub(crate) fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let len_lresult = SendMessageW(hwnd, WM_GETTEXTLENGTH, WPARAM(0), LPARAM(0)).0;
        assert!(len_lresult >= 0, "WM_GETTEXTLENGTH returned bogus value");
        let len = len_lresult as usize;
        let capacity = len + 1;
        let mut buffer: Vec<u16> = Vec::with_capacity(capacity);
        let len_copied = SendMessageW(
            hwnd,
            WM_GETTEXT,
            WPARAM(capacity),
            LPARAM(buffer.as_mut_ptr() as isize),
        )
        .0;
        assert!(len_copied >= 0);
        assert!((len_copied as usize) <= len);
        buffer.set_len(len_copied as usize);
        widestring::U16Str::from_slice(buffer.as_slice()).to_string_lossy()
    }
}

const STAP_ALLOW_NONCLIENT: u32 = 1 << 0;
const STAP_ALLOW_CONTROLS: u32 = 1 << 1;
const STAP_ALLOW_WEBCONTENT: u32 = 1 << 2;

pub(crate) const WM_NOTIFY: u32 = 0x004E;

fn clone_cell_opt_rc<T>(rc: &Cell<Option<Rc<T>>>) -> Option<Rc<T>> {
    let value = rc.take();
    let result = value.clone();
    rc.set(value);
    result
}

pub fn get_cursor_pos() -> POINT {
    unsafe {
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        pt
    }
}

pub(crate) fn get_instance() -> HMODULE {
    unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleA(None).unwrap() }
}

pub fn with<T, F: FnMut(&mut T)>(mut value: T, mut f: F) -> T {
    f(&mut value);
    value
}

pub trait With {
    fn with<F: FnMut(&mut Self)>(mut self, mut f: F) -> Self
    where
        Self: Sized,
    {
        f(&mut self);
        self
    }
}

impl<T> With for T {}

pub(crate) struct DeferWindowPosOp {
    hdwp: HDWP,
}

// https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-deferwindowpos
impl DeferWindowPosOp {
    pub fn begin(n: i32) -> Result<Self> {
        unsafe {
            let hdwp = BeginDeferWindowPos(n).unwrap();
            Ok(Self { hdwp })
        }
    }

    pub fn defer(
        &mut self,
        hwnd: HWND,
        hwnd_insert_after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: SET_WINDOW_POS_FLAGS,
    ) {
        unsafe {
            self.hdwp =
                DeferWindowPos(self.hdwp, hwnd, hwnd_insert_after, x, y, cx, cy, flags).unwrap();
        }
    }
}

impl Drop for DeferWindowPosOp {
    fn drop(&mut self) {
        unsafe {
            EndDeferWindowPos(self.hdwp);
        }
    }
}

#[derive(Clone)]
struct StuckToThread {
    #[cfg(debug_assertions)]
    thread_id: u32,
    not_send: PhantomData<*mut u8>,
}

assert_not_impl_any!(StuckToThread: Sync, Send, Copy);

impl StuckToThread {
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            thread_id: unsafe { GetCurrentThreadId() },
            not_send: PhantomData,
        }
    }

    #[cfg_attr(not(debug_assertions), inline(always))]
    #[cfg_attr(debug_assertions, inline(never))]
    pub fn check(&self) {
        #[cfg(debug_assertions)]
        {
            let this_thread_id = unsafe { GetCurrentThreadId() };
            debug_assert_eq!(
                this_thread_id, self.thread_id,
                "Expected this object to be used only on the thread that created it."
            );
        }
    }
}
