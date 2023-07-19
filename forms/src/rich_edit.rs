use crate::ControlState;
use crate::Form;
use std::mem::zeroed;
use std::rc::Rc;
use std::sync::Once;
use widestring::U16CString;
use windows::core::PCWSTR;
use windows::w;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::LibraryLoader::LoadLibraryW;
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;
use windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE;
use windows::Win32::UI::WindowsAndMessaging::WM_USER;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, ES_MULTILINE, WINDOW_EX_STYLE, WS_BORDER, WS_CHILD, WS_TABSTOP, WS_VISIBLE,
};

pub struct RichEdit {
    control: ControlState,
}

impl core::ops::Deref for RichEdit {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl RichEdit {
    pub fn new(parent: &ControlState) -> Rc<RichEdit> {
        load_rich_edit_dll();

        unsafe {
            let style =
                WINDOW_STYLE(ES_MULTILINE as _) | WS_VISIBLE | WS_CHILD | WS_BORDER | WS_TABSTOP;
            let ex_style = WINDOW_EX_STYLE(0);

            let handle = CreateWindowExW(
                ex_style,
                w!("RICHEDIT50W"),
                PCWSTR::null(),
                style,
                0,
                0,
                0,
                0,
                Some(&parent.handle()),
                None, // menu
                None, // instance
                None,
            );

            if handle.0 == 0 {
                panic!("Failed to create window");
            }

            Rc::new(RichEdit {
                control: ControlState::new(handle),
            })
        }
    }

    pub fn set_plain_text(&self, s: &str) {
        let ws = U16CString::from_str_truncate(s);
        unsafe {
            let set_text: SETTEXTEX = SETTEXTEX {
                flags: 0,
                codepage: SETTEXT_CODEPAGE_UNICODE,
            };

            SendMessageW(
                self.handle(),
                EM_SETTEXTEX,
                WPARAM(&set_text as *const _ as usize),
                LPARAM(ws.as_ptr() as isize),
            );
        }
    }

    // https://learn.microsoft.com/en-us/windows/win32/controls/em-setoptions
    fn set_option_bool(&self, mask: u32, value: bool) {
        unsafe {
            if value {
                SendMessageW(
                    self.handle(),
                    EM_SETOPTIONS,
                    WPARAM(ECOOP_OR as usize),
                    LPARAM(mask as isize),
                );
            } else {
                SendMessageW(
                    self.handle(),
                    EM_SETOPTIONS,
                    WPARAM(ECOOP_AND as usize),
                    LPARAM((!mask) as isize),
                );
            }
        }
    }

    fn get_option_bool(&self, mask: u32) -> bool {
        unsafe {
            let lresult = SendMessageW(self.handle(), EM_GETOPTIONS, WPARAM(0), LPARAM(0));
            (lresult.0 as u32) & mask != 0
        }
    }
}

// Edit control options
const ECO_AUTOWORDSELECTION: u32 = 0x00000001;
const ECO_AUTOVSCROLL: u32 = 0x00000040;
const ECO_AUTOHSCROLL: u32 = 0x00000080;
const ECO_NOHIDESEL: u32 = 0x00000100;
const ECO_READONLY: u32 = 0x00000800;
const ECO_WANTRETURN: u32 = 0x00001000;
const ECO_SAVESEL: u32 = 0x00008000;
const ECO_SELECTIONBAR: u32 = 0x01000000;
const ECO_VERTICAL: u32 = 0x00400000; // FE specific

macro_rules! eco_options {
    (
        $(
            $flag:expr, $getter:ident, $setter:ident;
        )*
    ) => {
        impl RichEdit {
            $(
                pub fn $setter(&self, value: bool) {
                    self.set_option_bool($flag, value);
                }
                pub fn $getter(&self) -> bool {
                    self.get_option_bool($flag)
                }
            )*
        }
    }
}

eco_options! {
    ECO_AUTOWORDSELECTION, get_auto_word_selection, set_auto_word_selection;
    ECO_AUTOVSCROLL, get_auto_vscroll, set_auto_vscroll;
    ECO_AUTOHSCROLL, get_auto_hscroll, set_auto_hscroll;
    ECO_NOHIDESEL, get_no_hide_selection, set_no_hide_selection;
    ECO_READONLY, get_read_only, set_read_only;
    ECO_WANTRETURN, get_want_return, set_want_return;
    ECO_SAVESEL, get_save_selection, set_save_selection;
    ECO_SELECTIONBAR, get_selection_bar, set_selection_bar;
    ECO_VERTICAL, get_vertical, set_vertical;

}

// ECO operations
#[allow(dead_code)]
const ECOOP_SET: u32 = 1;
const ECOOP_OR: u32 = 2;
const ECOOP_AND: u32 = 3;
#[allow(dead_code)]
const ECOOP_XOR: u32 = 4;

// https://learn.microsoft.com/en-us/windows/win32/api/richedit/ns-richedit-settextex
#[repr(C)]
struct SETTEXTEX {
    // Flags (see the ST_XXX defines)
    flags: u32,
    // Code page for translation (CP_ACP for sys default,
    //	1200 for Unicode, -1 for control default)
    codepage: u32,
}

const SETTEXT_CODEPAGE_UNICODE: u32 = 1200;

// see richedit.h
const EM_SETOPTIONS: u32 = WM_USER + 77;
const EM_GETOPTIONS: u32 = WM_USER + 78;
const EM_SETTEXTEX: u32 = WM_USER + 97;

pub fn load_rich_edit_dll() {
    ONCE_LOAD_RICH_EDIT_DLL.call_once(|| unsafe {
        let _ = LoadLibraryW(w!("msftedit.dll"));
    })
}

static ONCE_LOAD_RICH_EDIT_DLL: Once = Once::new();
