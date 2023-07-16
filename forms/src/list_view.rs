use super::*;

impl core::ops::Deref for ListView {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

pub struct ListView {
    control: ControlState,
}

const WC_LISTVIEW: &str = "SysListView32";

impl ListView {
    pub fn set_visible(&self, value: bool) {
        let style = self.control.get_window_style();
        let new_style = (style & !WS_VISIBLE) | (if value { WS_VISIBLE } else { WINDOW_STYLE(0) });
        self.control.set_window_style(new_style);
    }

    pub fn new(form: &Rc<Form>) -> Rc<ListView> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate(WC_LISTVIEW);
            let ex_style = WINDOW_EX_STYLE(0);
            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR::from_raw(class_name_wstr.as_ptr()),
                PCWSTR::from_raw(window_name.as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER | WS_TABSTOP,
                0,
                0,
                0,
                0,
                parent_window,
                HMENU(0),
                get_instance(),
                None,
            );

            if hwnd.0 == 0 {
                panic!("failed to create ListView window");
            }

            let state: Rc<ListView> = Rc::new(ListView {
                control: ControlState::new(hwnd),
            });
            form.invalidate_layout();

            state
        }
    }

    /// Iterates the indices of the selected items.
    pub fn iter_selected_items(&self) -> IterSelectedItems<'_> {
        IterSelectedItems {
            next: -1,
            list_view: self,
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setitemstate
    pub fn set_item_selected(&self, item: usize, value: bool) {
        unsafe {
            let mut lv_item: LVITEMW = zeroed();
            lv_item.stateMask = LVIS_SELECTED;
            lv_item.state = LIST_VIEW_ITEM_STATE_FLAGS((value as u32) * LVIS_SELECTED.0);
            SendMessageW(
                self.handle(),
                LVM_SETITEMSTATE,
                WPARAM(item),
                LPARAM(&lv_item as *const LVITEMW as isize),
            );
        }
    }

    pub fn set_all_selected(&self, value: bool) {
        let len = self.items_len();
        for i in 0..len {
            self.set_item_selected(i, value);
        }
    }

    // Columns
    pub fn add_column(&self, index: u32, width: i32, text: &str) {
        unsafe {
            let textw = WCString::from_str_truncate(text);
            let mut col: LVCOLUMNW = zeroed();
            col.mask |= LVCF_TEXT | LVCF_WIDTH;
            col.cx = width;
            col.pszText = PWSTR(textw.as_ptr() as *mut u16);
            SendMessageW(
                self.control.handle(),
                LVM_INSERTCOLUMNW,
                WPARAM(index as usize),
                LPARAM(&col as *const LVCOLUMNW as isize),
            );
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-deletecolumn
    pub fn delete_column(&self, index: usize) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                LVM_DELETECOLUMN,
                WPARAM(index),
                LPARAM(0),
            );
        }
    }

    // Appearance properties

    // https://docs.microsoft.com/en-us/windows/win32/controls/extended-list-view-styles

    #[allow(dead_code)]
    fn get_ex_style(&self) -> WINDOW_EX_STYLE {
        unsafe {
            WINDOW_EX_STYLE(
                SendMessageW(
                    self.control.handle(),
                    LVM_GETEXTENDEDLISTVIEWSTYLE,
                    WPARAM(0),
                    LPARAM(0),
                )
                .0 as u32,
            )
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setextendedlistviewstyle
    fn set_ex_style(&self, mask: u32, values: u32) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                WPARAM(mask as usize),
                LPARAM(values as isize),
            );
        }
    }

    fn set_ex_style_bool(&self, mask: u32, value: bool) {
        self.set_ex_style(mask, if value { mask } else { 0 });
    }

    fn get_ex_style_bool(&self, mask: u32) -> bool {
        self.get_ex_style().0 & mask != 0
    }

    pub fn set_full_row_select(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_FULLROWSELECT, value);
    }

    pub fn get_full_row_select(&self) -> bool {
        self.get_ex_style_bool(LVS_EX_FULLROWSELECT)
    }

    pub fn set_check_boxes(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_CHECKBOXES, value);
    }

    pub fn set_grid_lines(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_GRIDLINES, value);
    }

    pub fn set_double_buffer(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_DOUBLEBUFFER, value);
    }

    pub fn set_header_drag_drop(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_HEADERDRAGDROP, value);
    }

    pub fn set_multi_select(&self, value: bool) {
        self.control
            .set_window_style_flag(WINDOW_STYLE(LVS_SINGLESEL), !value);
    }

    pub fn set_show_sort_header(&self, value: bool) {
        self.control
            .set_window_style_flag(WINDOW_STYLE(LVS_NOSORTHEADER), !value);
    }

    pub fn set_edit_labels(&self, value: bool) {
        self.control
            .set_window_style_flag(WINDOW_STYLE(LVS_EDITLABELS), value);
    }

    /// Sets all items to the not-selected state.
    pub fn clear_selection(&self) {}

    /// Gets the number of items in the list view.
    ///
    /// See <https://docs.microsoft.com/en-us/windows/win32/controls/lvm-getitemcount>
    pub fn items_len(&self) -> usize {
        unsafe { SendMessageW(self.handle(), LVM_GETITEMCOUNT, WPARAM(0), LPARAM(0)).0 as usize }
    }

    pub fn delete_item(&self, item: usize) {
        unsafe {
            SendMessageW(self.handle(), LVM_DELETEITEM, WPARAM(item), LPARAM(0));
        }
    }

    pub fn delete_all_items(&self) {
        unsafe {
            SendMessageW(self.handle(), LVM_DELETEALLITEMS, WPARAM(0), LPARAM(0));
        }
    }

    pub fn get_item_text(&self, item: usize, subitem: usize) -> String {
        unsafe {
            let mut buffer: Vec<u16> = vec![0; 0x100];
            let mut lv_item: LVITEMW = zeroed();
            lv_item.iItem = item as i32;
            lv_item.iSubItem = subitem as i32;
            lv_item.pszText = PWSTR(buffer.as_mut_ptr());
            lv_item.cchTextMax = buffer.len() as i32;

            let mut len = SendMessageW(
                self.handle(),
                LVM_GETITEMTEXT,
                WPARAM(item),
                LPARAM(&mut lv_item as *mut _ as isize),
            )
            .0;
            assert!(len >= 0);
            if len as usize > buffer.len() {
                // Resize the buffer, try again.

                buffer.resize(len as usize, 0);
                lv_item.pszText = PWSTR(buffer.as_mut_ptr());
                lv_item.cchTextMax = buffer.len() as i32;

                len = SendMessageW(
                    self.handle(),
                    LVM_GETITEMTEXT,
                    WPARAM(item),
                    LPARAM(&mut lv_item as *mut _ as isize),
                )
                .0;
                assert!(len >= 0);
            }

            WStr::from_slice(&buffer[..len as usize]).to_string_lossy()
        }
    }

    /// https://docs.microsoft.com/en-us/windows/win32/controls/lvm-ensurevisible
    pub fn ensure_visible(&self, item: usize) {
        unsafe {
            SendMessageW(self.handle(), LVM_ENSUREVISIBLE, WPARAM(item), LPARAM(0));
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-insertitem
    pub fn insert_item(&self, text: &str) -> usize {
        unsafe {
            let len = self.items_len();

            let textw = WCString::from_str_truncate(text);
            let mut lv_item: LVITEMW = zeroed();
            lv_item.iItem = len as i32;
            lv_item.iSubItem = 0;
            lv_item.mask |= LVIF_TEXT;
            lv_item.pszText = PWSTR(textw.as_ptr() as *mut u16);
            SendMessageW(
                self.handle(),
                LVM_INSERTITEMW,
                WPARAM(0),
                LPARAM(&lv_item as *const _ as isize),
            )
            .0 as usize
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setitemtext
    pub fn set_subitem_text(&self, item: usize, subitem: usize, s: &str) {
        unsafe {
            let sw = WCString::from_str_truncate(s);
            let mut lv_item: LVITEMW = zeroed();
            lv_item.iSubItem = subitem as i32;
            lv_item.mask = LVIF_TEXT;
            lv_item.pszText = PWSTR(sw.as_ptr() as *mut u16);
            SendMessageW(
                self.handle(),
                LVM_SETITEMTEXTW,
                WPARAM(item),
                LPARAM(&lv_item as *const _ as isize),
            );
        }
    }

    pub fn set_item_text(&self, item: usize, s: &str) {
        self.set_subitem_text(item, 0, s);
    }

    /// Sets multiple attributes of an item in a single call.
    pub fn set_item(&self, item: usize, attributes: &SetItem) {}

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setitemstate
    pub fn set_item_state(
        &self,
        item: usize,
        subitem: usize,
        state: LIST_VIEW_ITEM_STATE_FLAGS,
        state_mask: LIST_VIEW_ITEM_STATE_FLAGS,
    ) {
        unsafe {
            let mut lv_item: LVITEMW = zeroed();
            lv_item.stateMask = state_mask;
            lv_item.state = state;
            lv_item.iSubItem = subitem as i32;

            SendMessageW(
                self.handle(),
                LVM_SETITEMSTATE,
                WPARAM(item),
                LPARAM(&lv_item as *const _ as isize),
            );
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setview
    pub fn set_mode(&self, mode: Mode) {
        unsafe {
            SendMessageW(
                self.handle(),
                LVM_SETVIEW,
                WPARAM(mode.to_native() as usize),
                LPARAM(0),
            );
        }
    }
}

/*
impl NotifyHandlerTrait for ListView {
    unsafe fn wm_notify(&self, _control_id: WPARAM, nmhdr: *mut NMHDR) -> NotifyResult {
        match (*nmhdr).code as i32 {
            // https://docs.microsoft.com/en-us/windows/win32/controls/nm-click-list-view
            NM_CLICK => {
                let item_ptr = nmhdr as *const NMITEMACTIVATE;
                let item = &*item_ptr;
                self.for_all_handlers(|h| {
                    if let ListViewHandler::Click(h) = h {
                        (h.handler)(item.into());
                    }
                });
                NotifyResult::NotConsumed
            }
            NM_DBLCLK => {
                trace!("list view NM_DBLCLK");
                NotifyResult::NotConsumed
            }
            NM_RCLICK => {
                let item_ptr = nmhdr as *const NMITEMACTIVATE;
                let item = &*item_ptr;
                self.for_all_handlers(|h| {
                    if let ListViewHandler::RClick(h) = h {
                        (h.handler)(item.into());
                    }
                });
                NotifyResult::NotConsumed
            }

            _ => {
                // trace!("unrecognized WM_NOTIFY");
                NotifyResult::NotConsumed
            }
        }
    }
}
*/

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    Details,
    Icon,
    List,
    SmallIcon,
    Tile,
}

impl Mode {
    fn to_native(self) -> u32 {
        match self {
            Mode::Details => LV_VIEW_DETAILS,
            Mode::Icon => LV_VIEW_ICON,
            Mode::List => LV_VIEW_LIST,
            Mode::SmallIcon => LV_VIEW_SMALLICON,
            Mode::Tile => LV_VIEW_TILE,
        }
    }
}

/// The set of selected items has changed.
pub struct SelectionChanged {
    pub num_items_selected: usize,
    pub first_item_selected: Option<usize>,
    pub focus_item: Option<usize>,
}

/// The user has double-clicked on an item.
pub struct ItemActivated {
    pub item: usize,
}

pub struct IterSelectedItems<'a> {
    next: isize,
    list_view: &'a ListView,
}

impl<'a> Iterator for IterSelectedItems<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        unsafe {
            let result = SendMessageW(
                self.list_view.control.handle(),
                LVM_GETNEXTITEM,
                WPARAM(self.next as usize),
                LPARAM(LVNI_SELECTED as isize),
            )
            .0;
            if result < 0 {
                return None;
            }
            self.next = result;
            Some(result as usize)
        }
    }
}
