use super::*;
use core::any::Any;

impl core::ops::Deref for TreeView {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

pub struct TreeView {
    control: ControlState,
    handlers: RefCell<Vec<Rc<TreeViewHandler>>>,

    // key is HTREEITEM
    items: RefCell<HashMap<isize, Rc<NodeState>>>,
}

#[derive(Clone, Debug, Default)]
pub struct TreeViewOptions {
    pub has_buttons: bool,
    pub has_lines: bool,
    pub always_show_selection: bool,
    pub show_lines_at_root: bool,
    pub full_row_select: bool,
    pub single_expand: bool,
    pub checkboxes: bool,
}

const WC_TREEVIEW: &str = "SysTreeView32";

impl TreeView {
    pub fn set_visible(&self, value: bool) {
        let style = self.control.get_window_style();
        let new_style = (style & !WS_VISIBLE) | (if value { WS_VISIBLE } else { WINDOW_STYLE(0) });
        self.control.set_window_style(new_style);
    }

    pub fn new(form: &Rc<Form>, options: &TreeViewOptions) -> Rc<TreeView> {
        unsafe {
            let parent_window = form.handle();
            let class_name_wstr = WCString::from_str_truncate(WC_TREEVIEW);
            let ex_style = 0;

            let mut style = WS_CHILD | WS_VISIBLE | WS_CHILDWINDOW | WS_BORDER | WS_TABSTOP;
            if options.has_lines {
                style |= WINDOW_STYLE(TVS_HASLINES);
            }
            if options.always_show_selection {
                style |= WINDOW_STYLE(TVS_SHOWSELALWAYS);
            }
            if options.show_lines_at_root {
                style |= WINDOW_STYLE(TVS_LINESATROOT);
            }
            if options.has_buttons {
                style |= WINDOW_STYLE(TVS_HASBUTTONS);
            }
            if options.full_row_select {
                style |= WINDOW_STYLE(TVS_FULLROWSELECT);
            }
            if options.single_expand {
                style |= WINDOW_STYLE(TVS_SINGLEEXPAND);
            }
            if options.checkboxes {
                style |= WINDOW_STYLE(TVS_CHECKBOXES);
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(ex_style),
                PCWSTR::from_raw(class_name_wstr.as_ptr() as *mut _),
                PCWSTR::from_raw(null_mut()),
                style,
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
                panic!("failed to create TreeView window");
            }

            let state: Rc<TreeView> = Rc::new(TreeView {
                control: ControlState::new(form, hwnd),
                handlers: RefCell::new(Vec::new()),
                items: RefCell::new(HashMap::new()),
            });
            form.invalidate_layout();

            let mut notify_handlers = form.notify_handlers.borrow_mut();
            let state_rc: Rc<TreeView> = Rc::clone(&state);
            notify_handlers.insert(
                state.handle().0,
                NotifyHandler {
                    handler: Rc::new(TreeViewNotifyShim { tree: state_rc }),
                },
            );
            state
        }
    }

    // Appearance properties

    #[allow(dead_code)]
    fn get_ex_style(&self) -> u32 {
        unsafe {
            SendMessageW(
                self.control.handle(),
                TVM_GETEXTENDEDSTYLE,
                WPARAM(0),
                LPARAM(0),
            )
            .0 as u32
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/tvm-setextendedstyle
    fn set_ex_style(&self, mask: u32, values: u32) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                TVM_SETEXTENDEDSTYLE,
                WPARAM(mask as usize),
                LPARAM(values as isize),
            );
        }
    }

    fn set_ex_style_flag(&self, mask: u32, value: bool) {
        self.set_ex_style(mask, if value { mask } else { 0 });
    }

    pub fn set_double_buffer(&self, value: bool) {
        self.set_ex_style_flag(TVS_EX_DOUBLEBUFFER, value);
    }

    pub fn insert_root(self: &Rc<Self>, item: &str) -> Result<TreeNode> {
        self.insert_at(TVI_ROOT, item)
    }

    fn insert_at(self: &Rc<Self>, parent_hitem: HTREEITEM, text: &str) -> Result<TreeNode> {
        unsafe {
            let mut item: TVINSERTSTRUCTW = zeroed();
            item.hParent = parent_hitem;
            item.hInsertAfter = TVI_LAST;

            let itemex = &mut item.Anonymous.itemex;
            itemex.hwnd = self.handle();
            itemex.mask = TVIF_TEXT;

            let text_wstr = WCString::from_str(text).unwrap();
            itemex.pszText = PWSTR(text_wstr.as_ptr() as *const _ as *mut _);

            let hitem = HTREEITEM(
                SendMessageW(
                    self.handle(),
                    TVM_INSERTITEM,
                    WPARAM(0),
                    LPARAM(&item as *const _ as isize),
                )
                .0,
            );
            if hitem.0 == 0 {
                return Err(Error::Windows(GetLastError()));
            }

            let state = Rc::new(NodeState {
                hitem,
                deleted: Cell::new(false),
                data: RefCell::new(None),
            });

            {
                let mut items = self.items.borrow_mut();
                items.insert(hitem.0, Rc::clone(&state));
            }

            Ok(TreeNode {
                tree: Rc::clone(self),
                state,
            })
        }
    }

    fn add_handler(&self, handler: TreeViewHandler) {
        let mut handlers = self.handlers.borrow_mut();
        handlers.push(Rc::new(handler));
    }

    pub fn on_selection_changed(&self, handler: EventHandler<SelectionChanged>) {
        self.add_handler(TreeViewHandler::SelectionChanged(handler));
    }

    fn for_all_handlers(&self, f: impl Fn(&TreeViewHandler)) {
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

pub struct NewItem<'a> {
    pub text: &'a str,
}

#[derive(Clone)]
pub struct TreeNode {
    tree: Rc<TreeView>,
    state: Rc<NodeState>,
}

struct NodeState {
    hitem: HTREEITEM, // native pointer to tree view node
    deleted: Cell<bool>,
    data: RefCell<Option<Box<dyn Any>>>,
}

impl TreeNode {
    pub fn insert_child(&self, item: &str) -> Result<TreeNode> {
        if self.state.deleted.get() {
            return Err(Error::ItemDeleted);
        }

        self.tree.insert_at(self.state.hitem, item)
    }

    pub fn data(&self) -> &RefCell<Option<Box<dyn Any>>> {
        &self.state.data
    }

    pub fn set_data_opt(&self, data: Option<Box<dyn Any>>) {
        *self.state.data.borrow_mut() = data;
    }

    pub fn set_data<T: Any>(&self, value: T) {
        let b = Box::new(value);
        *self.state.data.borrow_mut() = Some(b);
    }

    pub fn take_data(&self) -> Option<Box<dyn Any>> {
        let mut b = self.state.data.borrow_mut();
        b.take()
    }

    pub fn borrow_data(&self) -> std::cell::Ref<'_, Option<Box<dyn Any>>> {
        self.state.data.borrow()
    }

    pub fn borrow_data_mut(&self) -> std::cell::Ref<'_, Option<Box<dyn Any>>> {
        self.state.data.borrow()
    }

    pub fn delete(&self) {
        if self.state.deleted.get() {
            return;
        }

        // We need to recursively walk the subtree and delete _our_ side of
        // each tree node.

        let mut items = self.tree.items.borrow_mut();

        remove_items_rec(self.tree.handle(), self.state.hitem, &mut items);
        assert!(self.state.deleted.get());

        // This should delete hitem and all of the items below it.
        // Since we just pulled them out of the handle table _and_ set the
        // `deleted` field of each one, we should be good.
        unsafe {
            SendMessageW(
                self.tree.handle(),
                TVM_DELETEITEM,
                WPARAM(0),
                LPARAM(self.state.hitem.0),
            );
        }
    }

    // https://docs.microsoft.com/en-us/windows/win32/controls/tvm-expand
    pub fn expand(&self) {
        unsafe {
            if self.state.deleted.get() {
                return;
            }

            SendMessageW(
                self.tree.handle(),
                TVM_EXPAND,
                WPARAM(TVE_EXPAND.0 as usize),
                LPARAM(self.state.hitem.0),
            );
        }
    }

    pub fn collapse(&self) {
        unsafe {
            if self.state.deleted.get() {
                return;
            }

            SendMessageW(
                self.tree.handle(),
                TVM_EXPAND,
                WPARAM(TVE_COLLAPSE.0 as usize),
                LPARAM(self.state.hitem.0),
            );
        }
    }

    pub fn toggle(&self) {
        unsafe {
            if self.state.deleted.get() {
                return;
            }

            SendMessageW(
                self.tree.handle(),
                TVM_EXPAND,
                WPARAM(TVE_TOGGLE.0 as usize),
                LPARAM(self.state.hitem.0),
            );
        }
    }

    pub fn ensure_visible(&self) {
        unsafe {
            if self.state.deleted.get() {
                return;
            }

            SendMessageW(
                self.tree.handle(),
                TVM_ENSUREVISIBLE,
                WPARAM(0),
                LPARAM(self.state.hitem.0),
            );
        }
    }

    pub fn is_selected(&self) -> bool {
        unsafe {
            if self.state.deleted.get() {
                return false;
            }

            let state = SendMessageW(
                self.tree.handle(),
                TVM_GETITEMSTATE,
                WPARAM(self.state.hitem.0 as usize),
                LPARAM(TVIF_STATEEX.0 as isize),
            );
            (state.0 as u32 & TVIS_SELECTED.0) != 0
        }
    }
}

// This walks a tree of items and removes them from the `items` HashMap.
// This DOES NOT delete the items from the actual TreeView.
fn remove_items_rec(
    hwnd: HWND,
    hitem: HTREEITEM,
    items: &mut HashMap</*HTREEITEM*/ isize, Rc<NodeState>>,
) {
    unsafe {
        let first_child = HTREEITEM(
            SendMessageW(
                hwnd,
                TVM_GETNEXTITEM,
                WPARAM(TVGN_CHILD as usize),
                LPARAM(hitem.0),
            )
            .0,
        );
        let mut current_child = first_child;
        while current_child.0 != 0 {
            remove_items_rec(hwnd, current_child, items);
            let next_child = HTREEITEM(
                SendMessageW(
                    hwnd,
                    TVM_GETNEXTITEM,
                    WPARAM(TVGN_NEXT as usize),
                    LPARAM(current_child.0),
                )
                .0,
            );
            current_child = next_child;
        }
    }

    if let Some(removed_item) = items.remove(&hitem.0) {
        assert!(!removed_item.deleted.get());
        removed_item.deleted.set(true);
    }
}

struct TreeViewNotifyShim {
    tree: Rc<TreeView>,
}

impl NotifyHandlerTrait for TreeViewNotifyShim {
    unsafe fn wm_notify(&self, _control_id: WPARAM, nmhdr: *mut NMHDR) -> NotifyResult {
        match (*nmhdr).code as i32 {
            /*
                        // https://docs.microsoft.com/en-us/windows/win32/controls/nm-click-list-view
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
            */
            NM_DBLCLK => {
                trace!("NM_DBLCLK");
                NotifyResult::NotConsumed
            }

            #[allow(irrefutable_let_patterns)] // todo
            TVN_SELCHANGEDW => {
                trace!("TVN_SELCHANGEDW");
                self.tree.for_all_handlers(|h| {
                    if let TreeViewHandler::SelectionChanged(h) = h {
                        trace!("calling SelectionChanged event handler");

                        let selected_hitem = HTREEITEM(
                            SendMessageW(
                                self.tree.handle(),
                                TVM_GETNEXTITEM,
                                WPARAM(TVGN_CARET as usize),
                                LPARAM(0),
                            )
                            .0,
                        );

                        let selected_node: Option<TreeNode>;
                        if selected_hitem.0 != 0 {
                            let items = self.tree.items.borrow();
                            if let Some(n) = items.get(&selected_hitem.0) {
                                selected_node = Some(TreeNode {
                                    tree: Rc::clone(&self.tree),
                                    state: Rc::clone(n),
                                });
                            } else {
                                selected_node = None;
                            }
                        } else {
                            selected_node = None;
                        }

                        (h.handler)(SelectionChanged(selected_node));
                    }
                });

                NotifyResult::NotConsumed
            }

            TVN_SELCHANGINGW => {
                trace!("TVN_SELCHANGING");
                NotifyResult::NotConsumed
            }

            TVN_ITEMCHANGINGW => {
                trace!("TVN_ITEMCHANGINGW");
                NotifyResult::NotConsumed
            }

            NM_SETCURSOR => {
                trace!("NM_SETCURSOR");
                NotifyResult::NotConsumed
            }

            _ => {
                trace!("treeview: unrecognized WM_NOTIFY {}", (*nmhdr).code as i32);
                NotifyResult::NotConsumed
            }
        }
    }
}

pub struct SelectionChanged(pub Option<TreeNode>);

enum TreeViewHandler {
    SelectionChanged(EventHandler<SelectionChanged>),
    // ItemActivated(EventHandler<ItemActivated>),
    // Click(EventHandler<ItemActivate>),
    // RClick(EventHandler<ItemActivate>),
}
