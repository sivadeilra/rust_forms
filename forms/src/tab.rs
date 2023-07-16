use super::*;
use std::sync::Once;
use windows::w;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};

pub struct TabControl {
    control: ControlState,
    tabs: RefCell<Vec<Tab>>,
}

const TAB_WNDCLASS_NAME: PCWSTR = w!("rust_forms.tab");

struct Tab {
    hwnd: HWND,
    pane: Rc<TabPane>,
}

pub struct TabPane {
    pub(crate) layout: RefCell<Option<Layout>>,
    pub(crate) layout_is_valid: Cell<bool>,
    control: ControlState,
}

impl TabPane {
    pub fn set_layout(&self, layout: Layout) {
        *self.layout.borrow_mut() = Some(layout);
    }
}

impl core::ops::Deref for TabPane {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl core::ops::Deref for TabControl {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl TabControl {
    pub fn new(parent: &Rc<Form>) -> Rc<Self> {
        register_class_lazy();

        unsafe {
            let ex_style = WINDOW_EX_STYLE(0);
            let style = WS_CHILD | WS_CLIPSIBLINGS | WS_VISIBLE;

            let hwnd = CreateWindowExW(
                ex_style,
                WC_TABCONTROL,
                w!(""),
                style,
                0,   // x
                0,   // y
                400, // width
                400, // height
                parent.handle(),
                HMENU(0), // hmenu
                None,     // instance
                None,     // lpparam
            );
            if hwnd.0 == 0 {
                panic!("Failed to create tabs");
            }

            let rc = Rc::new(Self {
                control: ControlState::new(hwnd),
                tabs: RefCell::new(Vec::new()),
            });

            if true {
                let tab_control_ptr: *const TabControl = &*rc;

                SetWindowSubclass(
                    hwnd,
                    Some(tab_control_subclass_proc),
                    0, // subclass_id
                    tab_control_ptr as usize,
                );
            }

            parent.tab_controls.borrow_mut().push(Rc::downgrade(&rc));

            rc
        }
    }

    pub fn add_tab(&self, item_index: u32, label: &str) -> Rc<TabPane> {
        unsafe {
            let ex_style = WINDOW_EX_STYLE(0);
            let style = WS_CHILD | WS_CLIPSIBLINGS | WS_VISIBLE;

            let tab_hwnd = CreateWindowExW(
                ex_style,
                TAB_WNDCLASS_NAME,
                w!(""),
                style,
                50,  // x
                50,  // y
                400, // width
                400, // height
                self.handle(),
                HMENU(0), // hmenu
                None,     // instance
                None,     // lpparam
            );
            if tab_hwnd.0 == 0 {
                panic!("Failed to create tabs");
            }

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

            let pane = Rc::new(TabPane {
                layout: Default::default(),
                layout_is_valid: Cell::new(false),
                control: ControlState::new(tab_hwnd),
            });

            {
                let mut tabs = self.tabs.borrow_mut();
                tabs.push(Tab {
                    hwnd: tab_hwnd,
                    pane: Rc::clone(&pane),
                });
            }

            self.sync_visible();

            pane
        }
    }

    pub fn sync_visible(&self) {
        unsafe {
            debug!("TabControl::sync_visible");

            let client_rect = self.get_client_rect();

            let mut inner_client_rect = client_rect.clone();
            SendMessageW(
                self.handle(),
                TCM_ADJUSTRECT,
                WPARAM(0),
                LPARAM(&mut inner_client_rect as *mut RECT as isize),
            );
            debug!("TabControl: client rect: {:?}", client_rect);
            debug!("TabControl: inner area:  {:?}", inner_client_rect);

            let cur_sel = SendMessageW(self.handle(), TCM_GETCURSEL, WPARAM(0), LPARAM(0));
            let tabs = self.tabs.borrow();

            for (i, tab) in tabs.iter().enumerate() {
                let show_it = i as u32 == cur_sel.0 as u32;
                ShowWindow(tab.hwnd, if show_it { SW_SHOW } else { SW_HIDE });

                let mut deferred_placer = DeferredLayoutPlacer::new(50);

                if show_it {
                    if !tab.pane.layout_is_valid.get() || true {
                        let mut layout = tab.pane.layout.borrow_mut();
                        if let Some(layout) = &mut *layout {
                            layout.place(
                                &mut deferred_placer,
                                client_rect.left,
                                client_rect.top,
                                client_rect.right - client_rect.left,
                                client_rect.bottom - client_rect.top,
                            );
                        }

                        // Place the pane window within the tab control, using inner_client_rect
                        // because we want it to align to the "display area" of the tab control.
                        if true {
                            let x = inner_client_rect.left;
                            let y = inner_client_rect.top;
                            let width = inner_client_rect.right - inner_client_rect.left;
                            let height = inner_client_rect.bottom - inner_client_rect.top;
                            debug!(
                                "positioning pane: x {}, y {}, width {}, height {}",
                                inner_client_rect.left, inner_client_rect.top, width, height
                            );

                            // For some reason, using deferred window positioning is not working
                            // when setting the placement for the pane window.  Using SetWindowPos
                            // works, though.
                            if true {
                                SetWindowPos(
                                    tab.pane.handle(),
                                    HWND(0),
                                    x,
                                    y,
                                    width,
                                    height,
                                    SWP_NOZORDER,
                                );
                            } else {
                                deferred_placer.place_control(
                                    &tab.pane.control,
                                    x,
                                    y,
                                    width,
                                    height,
                                );
                            }
                        }

                        let mut placement = zeroed();
                        GetWindowPlacement(tab.pane.handle(), &mut placement);
                        debug!("placement: {:?}", placement);

                        tab.pane.layout_is_valid.set(true);
                    }
                }

                drop(deferred_placer);
            }
        }
    }
}

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut FORM_CLASS_ATOM: ATOM = 0;

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let instance = get_instance();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = TAB_WNDCLASS_NAME;
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_WINDOW.0 + 1) as _);
        class_ex.lpfnWndProc = Some(tab_wndproc);
        class_ex.hCursor = LoadCursorW(HMODULE(0), IDC_ARROW).unwrap();
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        FORM_CLASS_ATOM = atom;
    });

    unsafe { FORM_CLASS_ATOM }
}

extern "system" fn tab_wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging as wm;

    unsafe {
        match message {
            _ => {}
        }

        DefWindowProcW(window, message, wparam, lparam)
    }
}

unsafe extern "system" fn tab_control_subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    assert!(!ref_data != 0);
    let this: &TabControl = &*(ref_data as *const TabControl);

    match message {
        _ => {}
    }

    let result = DefSubclassProc(hwnd, message, wparam, lparam);

    match message {
        WM_SIZE => {
            log::debug!("TabControl: WM_SIZE");
            this.sync_visible();
        }
        _ => {}
    }

    result
}
