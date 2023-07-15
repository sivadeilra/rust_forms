use windows::w;

use super::*;

pub struct TabControl {
    control: ControlState,
}

impl core::ops::Deref for TabControl {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}
impl TabControl {
    pub fn new(parent: &Rc<Form>) -> Rc<Self> {
        unsafe {
            let ex_style = WINDOW_EX_STYLE(0);
            let style = WS_CHILD | WS_CLIPSIBLINGS | WS_VISIBLE;

            let hwnd = CreateWindowExW(
                ex_style,
                WC_TABCONTROL,
                w!(""),
                style,
                0,  // x
                0,  // y
                10, // width
                10, // height
                parent.handle(),
                HMENU(0), // hmenu
                None,     // instance
                None,     // lpparam
            );
            if hwnd.0 == 0 {
                panic!("Failed to create tabs");
            }

            Rc::new(Self {
                control: ControlState::new(parent, hwnd),
            })
        }
    }

    pub fn add_tab(&self, item_index: u32, label: &str) {
        unsafe {
            let label_wstr = U16CString::from_str_truncate(label);
            let mut item: TCITEMW = core::mem::zeroed();
            item.mask = TCIF_TEXT;
            item.iImage = -1;
            item.pszText = PWSTR(label_wstr.as_ptr() as *mut _);

            SendMessageW(
                self.control.handle(),
                TCM_INSERTITEM,
                WPARAM(item_index as usize),
                LPARAM(&item as *const TCITEMW as isize),
            );
        }
    }
}
