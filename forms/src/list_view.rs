use super::*;
use core::mem::transmute;
use core::mem::zeroed;
use windows::Win32::Foundation::PWSTR;
use windows::Win32::UI::Controls::{self, LVCF_TEXT, LVCOLUMNW};
use windows::Win32::UI::WindowsAndMessaging::SendMessageW;

pub struct ListView {
    control: Control,
}

impl core::ops::Deref for ListView {
    type Target = Control;
    fn deref(&self) -> &Control {
        &self.control
    }
}

impl ListView {
    /*
        pub fn new(parent: &Form) {
            unsafe {
                let ex_style: u32 = 0;

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
                    form_alloc.as_mut() as *mut UnsafeCell<FormState> as *mut c_void,
                );

                if handle == 0 {
                    panic!("Failed to create window");
                }
            }
        }
    */

    pub fn insert_column(&self, index: usize, column: Column<'_>) {
        unsafe {
            let mut lv_col: LVCOLUMNW = zeroed();

            let text_wstr: U16CString;
            if let Some(text) = column.text {
                text_wstr = U16CString::from_str(text).unwrap();
                lv_col.pszText = transmute(text_wstr.as_ptr());
                lv_col.mask |= LVCF_TEXT;
            }

            SendMessageW(
                self.control.handle(),
                Controls::LVM_INSERTCOLUMNW,
                index as usize,
                &lv_col as *const LVCOLUMNW as isize,
            );
        }
    }

    pub fn insert_item(&self, item: Item<'_>) {
        unsafe {
            let mut lv_item: Controls::LVITEMW = zeroed();

            let text_wstr: U16CString;

            if let Some(text) = item.text {
                text_wstr = U16CString::from_str(text).unwrap();
                lv_item.pszText = transmute(text_wstr.as_ptr());
                lv_item.mask |= Controls::LVIF_TEXT;
            }

            if let Some(indent) = item.indent {
                lv_item.iIndent = indent;
                lv_item.mask |= Controls::LVIF_INDENT;
            }

            SendMessageW(
                self.control.handle(),
                Controls::LVM_INSERTITEM,
                0,
                &lv_item as *const Controls::LVITEMW as isize,
            );
        }
    }

    pub fn set_item_text(&self, item: usize, text: &str) {
        self.set_subitem_text(item, 0, text);
    }

    pub fn set_subitem_text(&self, item: usize, subitem: usize, text: &str) {
        unsafe {
            let text_wstr: U16CString = U16CString::from_str(text).unwrap();
            let mut lv_item: Controls::LVITEMW = zeroed();
            lv_item.pszText = transmute(text_wstr.as_ptr());
            lv_item.iSubItem = subitem as i32;
            SendMessageW(
                self.control.handle(),
                Controls::LVM_SETITEMTEXTW,
                item,
                &lv_item as *const Controls::LVITEMW as isize,
            );
        }
    }

    pub fn set_item_count(&self, n: usize) {
        unsafe {
            SendMessageW(self.control.handle(), Controls::LVM_SETITEMCOUNT, n, 0);
        }
    }
}

#[derive(Default)]
pub struct Item<'a> {
    pub text: Option<&'a str>,
    pub indent: Option<i32>,
}

impl<'a> Item<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text: Some(text),
            ..Self::default()
        }
    }
}

#[derive(Default)]
pub struct Column<'a> {
    pub text: Option<&'a str>,
    pub width: Option<i32>,
}

impl<'a> Column<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text: Some(text),
            width: None,
        }
    }
}
