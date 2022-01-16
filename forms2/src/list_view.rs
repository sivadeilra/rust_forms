use super::*;
use core::any::Any;
use widestring::U16CString;

#[derive(Clone)]
pub struct ListView {
    state: Rc<ListViewState>,
}

struct ListViewState {
    control: Rc<ControlState>,
    handlers: RefCell<Vec<ListViewHandler>>,
}

const WC_LISTVIEW: &str = "SysListView32";

impl ListView {
    fn handle(&self) -> HWND {
        self.state.control.handle
    }

    pub fn new(form: &Form) -> Self {
        unsafe {
            let parent_window = form.handle();

            let window_name = WCString::from_str_truncate("");
            let class_name_wstr = WCString::from_str_truncate(WC_LISTVIEW);

            let x = 0;
            let y = 0;
            let nwidth = 300;
            let nheight = 300;

            let ex_style = 0;

            let windowHandle = CreateWindowExW(
                ex_style,
                PWSTR(class_name_wstr.as_ptr() as *mut _),
                PWSTR(window_name.as_ptr() as *mut _),
                WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER,
                x,
                y,
                nwidth,
                nheight,
                parent_window,
                0 as HMENU,     // hmenu,
                get_instance(), // hinstance,
                null_mut(),     // &*state as *const ListViewState as *const c_void as *mut c_void,
            );

            if windowHandle == 0 {
                panic!("failed to create ListView window");
            }

            debug!("created list view window 0x{:x}", windowHandle);

            let control = Rc::new(ControlState {
                controls: Vec::new(),
                // parent: Rc::downgrade(&form.state),
                handle: windowHandle,
                layout: ControlLayout::default(),
                form: Rc::downgrade(&form.state),
            });

            let state: Rc<ListViewState> = Rc::new(ListViewState {
                handlers: RefCell::new(Vec::new()),
                control: control,
            });
            form.state.invalidate_layout();
            let mut controls = form.state.controls.borrow_mut();
            // controls.insert(this.handle(), Rc::clone(&this));
            Self { state }
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
                self.state.control.handle,
                LVM_INSERTCOLUMNW,
                index as WPARAM,
                &col as *const LVCOLUMNW as LPARAM,
            );
        }
    }
    pub fn remove_column(&self, index: u32) {}

    pub fn set_column_text(&self, column: usize, heading: &str) {}

    // Appearance properties

    // https://docs.microsoft.com/en-us/windows/win32/controls/extended-list-view-styles

    fn get_ex_style(&self) -> u32 {
        unsafe {
            SendMessageW(
                self.state.control.handle,
                LVM_GETEXTENDEDLISTVIEWSTYLE,
                0,
                0,
            ) as u32
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-setextendedlistviewstyle
    fn set_ex_style(&self, mask: u32, values: u32) {
        unsafe {
            SendMessageW(
                self.state.control.handle,
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
        self.state
            .control
            .set_window_style_flag(LVS_SINGLESEL, !value);
    }

    pub fn set_show_sort_header(&self, value: bool) {
        self.state
            .control
            .set_window_style_flag(LVS_NOSORTHEADER, !value);
    }

    pub fn set_edit_labels(&self, value: bool) {
        self.state
            .control
            .set_window_style_flag(LVS_EDITLABELS, value);
    }

    /// Sets all items to the not-selected state.
    pub fn clear_selection(&self) {}

    /// Gets the number of items in the list view.
    pub fn items_len(&self) -> usize {
        todo!()
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

    pub fn get_item_data(&self, pos: usize) -> Option<&Box<dyn Any>> {
        todo!();
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/lvm-insertitem
    pub fn insert_item(&self, text: &str) {
        unsafe {
            let textw = WCString::from_str_truncate(text);
            let mut lv_item: LVITEMW = zeroed();
            lv_item.iSubItem = 0;
            lv_item.iItem = 0;
            lv_item.mask |= LVIF_TEXT;
            lv_item.pszText = PWSTR(textw.as_ptr() as *mut u16);
            SendMessageW(
                self.handle(),
                LVM_INSERTITEMW,
                0,
                &lv_item as *const _ as LPARAM,
            );
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
            lv_item.state = state_mask;
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
            let result = SendMessageW(self.handle(), LVM_SETVIEW, mode.to_native() as WPARAM, 0);
            debug!("LV_SETVIEW result: {}", result);
        }
    }

    // Events
    pub fn on_selection_changed(&self, handler: EventHandler<SelectionChanged>) {}
    pub fn on_item_activated(&self, handler: EventHandler<ItemActivated>) {}
}

pub struct SetItem<'a> {
    pub text: Option<&'a str>,
}

pub struct ItemRef<'a> {
    list_view: &'a ListView,
    index: usize,
}

impl<'a> ItemRef<'a> {}

enum ListViewHandler {
    SelectionChanged(EventHandler<SelectionChanged>),
    ItemActivated(EventHandler<ItemActivated>),
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
