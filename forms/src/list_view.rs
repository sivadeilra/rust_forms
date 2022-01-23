use super::*;

impl core::ops::Deref for ListView {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

pub struct ListView {
    control: ControlState,
    handlers: RefCell<Vec<Rc<ListViewHandler>>>,
}

const WC_LISTVIEW: &str = "SysListView32";

impl ListView {
    pub fn set_visible(&self, value: bool) {
        let style = self.control.get_window_style();
        let new_style = (style & !WS_VISIBLE) | (if value { WS_VISIBLE } else { 0 });
        self.control.set_window_style(new_style);
    }

    pub fn new(form: &Rc<Form>) -> Rc<ListView> {
        unsafe {
            let parent_window = form.handle();
            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate(WC_LISTVIEW);
            let ex_style = 0;
            let hwnd = CreateWindowExW(
                ex_style,
                PWSTR(class_name_wstr.as_ptr() as *mut _),
                PWSTR(window_name.as_ptr() as *mut _),
                WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER,
                0,
                0,
                0,
                0,
                parent_window,
                0 as HMENU,
                get_instance(),
                null_mut(),
            );

            if hwnd == 0 {
                panic!("failed to create ListView window");
            }

            debug!("created list view window 0x{:x}", hwnd);

            let state: Rc<ListView> = Rc::new(ListView {
                control: ControlState {
                    handle: hwnd,
                    layout: RefCell::new(ControlLayout::default()),
                    form: Rc::downgrade(&form),
                },
                handlers: RefCell::new(Vec::new()),
            });
            form.invalidate_layout();

            let mut notify_handlers = form.notify_handlers.borrow_mut();
            let state_rc: Rc<ListView> = Rc::clone(&state);
            notify_handlers.insert(state.handle(), NotifyHandler { handler: state_rc });
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
            lv_item.state = (value as u32) * LVIS_SELECTED;
            SendMessageW(
                self.handle(),
                LVM_SETITEMSTATE,
                item as WPARAM,
                &lv_item as *const LVITEMW as LPARAM,
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
                index as WPARAM,
                &col as *const LVCOLUMNW as LPARAM,
            );
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-deletecolumn
    pub fn delete_column(&self, index: usize) {
        unsafe {
            SendMessageW(self.control.handle(), LVM_DELETECOLUMN, index, 0);
        }
    }

    // pub fn set_column_text(&self, column: usize, heading: &str) {}

    // Appearance properties

    // https://docs.microsoft.com/en-us/windows/win32/controls/extended-list-view-styles

    #[allow(dead_code)]
    fn get_ex_style(&self) -> u32 {
        unsafe { SendMessageW(self.control.handle(), LVM_GETEXTENDEDLISTVIEWSTYLE, 0, 0) as u32 }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setextendedlistviewstyle
    fn set_ex_style(&self, mask: u32, values: u32) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                mask as WPARAM,
                values as LPARAM,
            );
        }
    }

    fn set_ex_style_bool(&self, mask: u32, value: bool) {
        self.set_ex_style(mask, if value { mask } else { 0 });
    }

    pub fn set_full_row_select(&self, value: bool) {
        self.set_ex_style_bool(LVS_EX_FULLROWSELECT, value);
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
        self.control.set_window_style_flag(LVS_SINGLESEL, !value);
    }

    pub fn set_show_sort_header(&self, value: bool) {
        self.control.set_window_style_flag(LVS_NOSORTHEADER, !value);
    }

    pub fn set_edit_labels(&self, value: bool) {
        self.control.set_window_style_flag(LVS_EDITLABELS, value);
    }

    /// Sets all items to the not-selected state.
    pub fn clear_selection(&self) {}

    /// Gets the number of items in the list view.
    ///
    /// See <https://docs.microsoft.com/en-us/windows/win32/controls/lvm-getitemcount>
    pub fn items_len(&self) -> usize {
        unsafe { SendMessageW(self.handle(), LVM_GETITEMCOUNT, 0, 0) as usize }
    }

    pub fn delete_item(&self, item: usize) {
        unsafe {
            SendMessageW(self.handle(), LVM_DELETEITEM, item, 0);
        }
    }

    pub fn delete_all_items(&self) {
        unsafe {
            SendMessageW(self.handle(), LVM_DELETEALLITEMS, 0, 0);
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
                item as WPARAM,
                &mut lv_item as *mut _ as LPARAM,
            );
            assert!(len >= 0);
            if len as usize > buffer.len() {
                // Resize the buffer, try again.

                buffer.resize(len as usize, 0);
                lv_item.pszText = PWSTR(buffer.as_mut_ptr());
                lv_item.cchTextMax = buffer.len() as i32;

                len = SendMessageW(
                    self.handle(),
                    LVM_GETITEMTEXT,
                    item as WPARAM,
                    &mut lv_item as *mut _ as LPARAM,
                );
                assert!(len >= 0);
            }

            WStr::from_slice(&buffer[..len as usize]).to_string_lossy()
        }
    }

    /// https://docs.microsoft.com/en-us/windows/win32/controls/lvm-ensurevisible
    pub fn ensure_visible(&self, item: usize) {
        unsafe {
            SendMessageW(self.handle(), LVM_ENSUREVISIBLE, item as WPARAM, 0);
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
                0,
                &lv_item as *const _ as LPARAM,
            ) as usize
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
                item as WPARAM,
                &lv_item as *const _ as LPARAM,
            );
        }
    }

    pub fn set_item_text(&self, item: usize, s: &str) {
        self.set_subitem_text(item, 0, s);
    }

    /// Sets multiple attributes of an item in a single call.
    pub fn set_item(&self, item: usize, attributes: &SetItem) {}

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setitemstate
    pub fn set_item_state(&self, item: usize, subitem: usize, state: u32, state_mask: u32) {
        unsafe {
            let mut lv_item: LVITEMW = zeroed();
            lv_item.stateMask = state_mask;
            lv_item.state = state;
            lv_item.iSubItem = subitem as i32;

            SendMessageW(
                self.handle(),
                LVM_SETITEMSTATE,
                item as WPARAM,
                &lv_item as *const _ as LPARAM,
            );
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setview
    pub fn set_view(&self, mode: Mode) {
        unsafe {
            SendMessageW(self.handle(), LVM_SETVIEW, mode.to_native() as WPARAM, 0);
        }
    }

    fn add_handler(&self, handler: ListViewHandler) {
        let mut handlers = self.handlers.borrow_mut();
        handlers.push(Rc::new(handler));
    }

    // Events
    #[cfg(todo)]
    pub fn on_selection_changed(&self, handler: EventHandler<SelectionChanged>) {
        self.add_handler(ListViewHandler::SelectionChanged(handler));
    }

    pub fn on_item_activated(&self, handler: EventHandler<ItemActivated>) {
        self.add_handler(ListViewHandler::ItemActivated(handler));
    }

    pub fn on_click(&self, handler: EventHandler<ItemActivate>) {
        self.add_handler(ListViewHandler::Click(handler));
    }

    pub fn on_rclick(&self, handler: EventHandler<ItemActivate>) {
        self.add_handler(ListViewHandler::RClick(handler));
    }

    fn for_all_handlers(&self, f: impl Fn(&ListViewHandler)) {
        let mut i = 0;
        loop {
            let handlers = self.handlers.borrow();
            if i >= handlers.len() {
                break;
            }
            let h = Rc::clone(&handlers[i]);
            i += 1;
            drop(handlers);
            f(&h);
        }
    }
}

impl NotifyHandlerTrait for ListView {
    unsafe fn wm_notify(&self, _control_id: WPARAM, nmhdr: *mut NMHDR) -> LRESULT {
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
                0
            }
            NM_DBLCLK => {
                trace!("list view NM_DBLCLK");
                0
            }
            NM_RCLICK => {
                let item_ptr = nmhdr as *const NMITEMACTIVATE;
                let item = &*item_ptr;
                self.for_all_handlers(|h| {
                    if let ListViewHandler::RClick(h) = h {
                        (h.handler)(item.into());
                    }
                });
                0
            }

            _ => {
                // trace!("unrecognized WM_NOTIFY");
                0
            }
        }
    }
}

pub struct SetItem<'a> {
    pub text: Option<&'a str>,
}

enum ListViewHandler {
    SelectionChanged(EventHandler<SelectionChanged>),
    ItemActivated(EventHandler<ItemActivated>),
    Click(EventHandler<ItemActivate>),
    RClick(EventHandler<ItemActivate>),
}

#[derive(Clone)]
pub struct ItemActivate {
    pub item: isize,
    pub subitem: isize,
    pub point: POINT,
}

impl From<&NMITEMACTIVATE> for ItemActivate {
    fn from(value: &NMITEMACTIVATE) -> Self {
        Self {
            item: value.iItem as isize,
            subitem: value.iSubItem as isize,
            point: value.ptAction,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    Details,
    Icon,
    List,
    SmallIcon,
    Tile,
}

impl Mode {
    fn to_native(&self) -> u32 {
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
                self.next as WPARAM,
                LVNI_SELECTED as LPARAM,
            );
            if result < 0 {
                return None;
            }
            self.next = result;
            Some(result as usize)
        }
    }
}
