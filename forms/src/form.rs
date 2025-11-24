use super::*;
use crate::dbg::message_str;
use crate::msg::Msg;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use std::cell::OnceCell;
use std::ops::Deref;
use std::sync::{Once, OnceLock};
use tracing::debug;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::WindowsAndMessaging as wm;

mod builder;
mod mdi;

pub use builder::*;

/// A top-level window.
#[derive(Clone)]
pub struct Form {
    pub(crate) rc: Rc<FormState>,
}

pub struct MdiClient {
    control: Rc<ControlState>,
}

impl Deref for MdiClient {
    type Target = Rc<ControlState>;

    fn deref(&self) -> &Self::Target {
        &self.control
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) enum MdiMode {
    /// Normal top-level window
    None,
    /// MDI frame
    Frame,
    /// MDI child
    Child,
}

pub(crate) struct FormState {
    pub(crate) app: App,

    stuck: StuckToThread,

    mdi_mode: MdiMode,

    pub(crate) mdi_client: Option<MdiClient>,

    pub(crate) control: Rc<ControlState>,
    quit_on_close: Option<i32>,

    is_layout_valid: Cell<bool>,
    layout_min_size: Cell<(i32, i32)>,

    pub(crate) layout: RefCell<Option<Layout>>,
    pub(crate) style: Rc<Style>,
    pub(crate) background_brush: RefCell<Option<Brush>>,
    pub(crate) background_color: Cell<ColorRef>,

    command_handler: OnceCell<Box<dyn Fn(ControlId, Command)>>,

    status_bar: Cell<Option<StatusBar>>,

    pub(crate) tab_controls: RefCell<Vec<std::rc::Weak<TabControl>>>,
}

assert_not_impl_any!(Form: Send, Sync);

#[allow(dead_code)]
pub(crate) trait MessageHandlerTrait: 'static {
    fn wm_command(&self, control_id: u16, notify_code: u16) -> LRESULT {
        let _ = (control_id, notify_code);
        LRESULT(0)
    }

    fn handle_message(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let _ = (msg, wparam, lparam);
        LRESULT(0)
    }
}

impl std::ops::Deref for Form {
    type Target = Rc<ControlState>;
    fn deref(&self) -> &Self::Target {
        &self.rc.control
    }
}

impl Form {
    pub fn app(&self) -> &App {
        &self.rc.app
    }

    pub(crate) fn handle(&self) -> HWND {
        self.rc.stuck.check();
        self.rc.control.handle()
    }

    pub fn show_window(&self) {
        self.rc.stuck.check();
        self.rc.ensure_layout_valid();
        unsafe {
            _ = ShowWindow(self.handle(), SW_SHOW);
        }
    }

    pub fn set_title(&self, text: &str) {
        self.rc.stuck.check();
        set_window_text(self.handle(), text);
    }

    pub fn style(&self) -> &Style {
        &self.rc.style
    }

    pub fn set_menu(&self, menu: Option<Menu>) {
        self.rc.stuck.check();
        unsafe {
            if let Some(menu) = menu {
                let hmenu = menu.extract();
                if SetMenu(self.handle(), Some(hmenu)).is_err() {
                    warn!("failed to set menu for form: {:?}", GetLastError());
                }
            } else {
                if SetMenu(self.handle(), None).is_ok() {
                    trace!("cleared menu for form");
                } else {
                    warn!("failed to clear menu for form");
                }
            }
        }
    }

    pub fn create_status_bar(&self) -> StatusBar {
        self.rc.stuck.check();
        let sb = if let Some(sb) = self.rc.status_bar.take() {
            sb
        } else {
            StatusBar::new(self)
        };
        self.rc.status_bar.set(Some(sb.clone()));
        sb
    }

    pub fn get_status_bar(&self) -> Option<StatusBar> {
        self.rc.stuck.check();
        if let Some(sb) = self.rc.status_bar.take() {
            self.rc.status_bar.set(Some(sb.clone()));
            Some(sb)
        } else {
            None
        }
    }

    pub fn set_font(&self, font: Rc<Font>) {
        unsafe {
            SendMessageW(
                self.handle(),
                WM_SETFONT,
                Some(WPARAM(font.hfont.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }

    pub fn enable(&self, value: bool) {
        unsafe {
            _ = EnableWindow(self.handle(), value);
        }
    }

    pub fn command_handler<F>(&self, handler: F)
    where
        F: Fn(ControlId, Command) + 'static,
    {
        let result = self.rc.command_handler.set(Box::new(handler));
        assert!(
            result.is_ok(),
            "cannot call command_handler() more than once"
        );
    }

    /*
    pub fn notify_handler<F>(&self, handler: F)
    where
        F: Fn(&Notify) + 'static,
    {
        let result = self.rc.notify_handler.set(Box::new(handler));
        assert!(
            result.is_ok(),
            "cannot call notify_handler() more than once"
        );
    }
    */

    pub fn mdi_client(&self) -> Option<&MdiClient> {
        self.rc.mdi_client.as_ref()
    }
}

impl FormState {
    pub(crate) fn invalidate_layout(&self) {
        self.stuck.check();
        self.is_layout_valid.set(false);
    }

    fn ensure_layout_valid(&self) {
        self.stuck.check();
        if self.is_layout_valid.get() {
            trace!("layout is already valid");
            return;
        }

        unsafe {
            let mut sb_height = 0;
            if let Some(sb) = self.status_bar.take() {
                self.status_bar.set(Some(sb.clone()));
                SendMessageW(sb.handle(), WM_SIZE, None, None);
                let mut sb_rect: RECT = zeroed();
                _ = GetClientRect(sb.handle(), &mut sb_rect);
                sb_height = sb_rect.bottom - sb_rect.top;
            }

            let mut client_rect: RECT = zeroed();
            if GetClientRect(self.control.handle(), &mut client_rect).is_ok() {
                trace!(
                    "running layout, rect: {},{} - {},{}",
                    client_rect.left,
                    client_rect.top,
                    client_rect.right,
                    client_rect.bottom
                );

                let layout_opt = self.layout.borrow();
                if let Some(layout) = &*layout_opt {
                    let min_size = layout.get_min_size();
                    self.layout_min_size.set(min_size);

                    let mut layout_height = client_rect.bottom - client_rect.top;
                    if layout_height >= sb_height {
                        layout_height -= sb_height;
                    }

                    let mut deferred_placer = DeferredLayoutPlacer::new(10);

                    layout.place(
                        &mut deferred_placer,
                        client_rect.left,
                        client_rect.top,
                        client_rect.right - client_rect.left,
                        layout_height, // client_rect.bottom - client_rect.top,
                    );
                    drop(deferred_placer);
                }
                self.is_layout_valid.set(true);
            } else {
                warn!("failed to get client rect");
            }
        }
    }
}

impl Form {
    pub fn set_layout(&self, layout: Layout) {
        self.rc.stuck.check();
        let mut layout_borrow = self.rc.layout.borrow_mut();
        *layout_borrow = Some(layout);
        drop(layout_borrow);

        self.rc.invalidate_layout();
        self.rc.ensure_layout_valid();
    }

    pub fn show_modal(&self) {
        self.show_modal_under(None)
    }

    pub fn show_modal_under(&self, parent: Option<&Form>) {
        self.rc.stuck.check();

        let disabler: Option<DisabledFormScope> = if let Some(p) = parent {
            unsafe {
                _ = EnableWindow(p.handle(), false);
            }
            Some(DisabledFormScope { form: p.handle() })
        } else {
            None
        };

        self.show_window();
        self.event_loop();

        drop(disabler);
    }

    fn event_loop(&self) {
        unsafe {
            loop {
                let mut msg: MSG = zeroed();
                let ret = GetMessageW(&mut msg, None, 0, 0).0;
                if ret < 0 {
                    debug!("event loop: GetMessageW returned {}, quitting", ret);
                    break;
                }

                // debug!("wm 0x{:04x}", msg.message);

                if msg.message == WM_QUIT {
                    debug!("found WM_QUIT, quitting");
                    break;
                }

                if IsDialogMessageW(self.handle(), &msg).into() {
                    continue;
                }

                _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

struct DisabledFormScope {
    pub(crate) form: HWND,
}

impl Drop for DisabledFormScope {
    fn drop(&mut self) {
        unsafe {
            _ = EnableWindow(self.form, true);
        }
    }
}

static REGISTER_CLASS_ONCE: OnceLock<ATOM> = OnceLock::new();

const FORM_CLASS_NAME: &str = "RustForms_Form";

fn register_class_lazy() -> ATOM {
    *REGISTER_CLASS_ONCE.get_or_init(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(FORM_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_mut_ptr());
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_BTNFACE.0 + 1) as _);
        class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        atom
    })
}

static MDI_CHILD_CLASS_ATOM: OnceLock<ATOM> = OnceLock::new();

const MDI_CHILD_CLASS_NAME: &str = "RustForms_MdiChildWindow";

fn register_mdi_child_lazy() -> ATOM {
    *MDI_CHILD_CLASS_ATOM.get_or_init(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(MDI_CHILD_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_mut_ptr());
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_BTNFACE.0 + 1) as _);
        class_ex.lpfnWndProc = Some(form_wndproc_mdi_child);
        // class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register MDI child window class");
        }
        atom
    })
}

#[cfg(false)]
extern "system" fn form_wndproc_mdi_frame(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    debug!("form_wndproc_mdi_frame: message {message:#4x}");

    unsafe {
        match message {
            wm::WM_MDIACTIVATE => {}

            wm::WM_GETMINMAXINFO => {
                let min_max: *mut MINMAXINFO = lparam.0 as *mut MINMAXINFO;
                min_max.write(MINMAXINFO {
                    ptMinTrackSize: POINT { x: 400, y: 400 },
                    ptMaxTrackSize: POINT { x: 10000, y: 10000 },
                    ..Default::default()
                });
                return LRESULT(0);
            }

            wm::WM_SIZE => {
                let mut client_rect: RECT = core::mem::zeroed();
                _ = GetClientRect(window, &mut client_rect);
                _ = SetWindowPos(
                    mdi_client_hwnd,
                    None,
                    0,
                    0,
                    client_rect.right,
                    client_rect.bottom,
                    SWP_NOZORDER,
                );
            }

            _ => {}
        }

        DefFrameProcW(window, None, message, wparam, lparam)
    }
}

extern "system" fn form_wndproc_mdi_child(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            wm::WM_CREATE => {
                let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut CREATESTRUCTW;
                assert!(!create_struct.is_null());

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                debug!(
                    "WM_CREATE for MDI child, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                /*
                let mdi_create_struct: &MDICREATESTRUCTW =
                    &*(create_params as *const MDICREATESTRUCTW);

                let form_state: *const FormState = mdi_create_struct.lParam.0 as *const FormState;

                debug!(?form_state, "the for-reals Form pointer");

                SetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0), form_state as isize);
                */
                return LRESULT(1);
            }

            wm::WM_CLOSE => {
                debug!("MDI child window received WM_CLOSE; ignoring");
                return LRESULT(0);
            }

            _ => {}
        }

        form_wndproc(window, message, wparam, lparam)
    }
}

extern "system" fn form_wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            wm::WM_CREATE => {
                let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut CREATESTRUCTW;
                assert!(!create_struct.is_null());

                debug!(
                    "WM_CREATE, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                /*

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                SetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0), create_params as isize);
                */
                return LRESULT(1);
            }

            _ => {}
        }

        let form_ptr: isize = GetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0));
        if form_ptr == 0 {
            debug!("form_wndproc: lparam is null, msg {:04x}", message);
            return DefWindowProcW(window, message, wparam, lparam);
        }

        let form: &FormState = &*(form_ptr as *const FormState);

        let app: &App = &form.app;

        match message {
            wm::WM_PAINT => {
                debug!("WM_PAINT");
                // ValidateRect(window, std::ptr::null());

                let mut ps: PAINTSTRUCT = core::mem::zeroed();
                let hdc: HDC = BeginPaint(window, &mut ps);

                _ = EndPaint(window, &ps);

                return LRESULT(0);
            }

            wm::WM_CLOSE => {
                if let Some(exit_code) = form.quit_on_close {
                    debug!("WM_CLOSE: posting quit message");
                    post_quit_message(exit_code);
                } else {
                    debug!("WM_CLOSE: not posting quit message");
                }
            }

            wm::WM_DESTROY => {
                debug!("WM_DESTROY");
                return LRESULT(0);
            }

            wm::WM_GETMINMAXINFO => {
                let min_max: *mut MINMAXINFO = lparam.0 as *mut MINMAXINFO;
                min_max.write(MINMAXINFO {
                    ptMinTrackSize: POINT { x: 400, y: 400 },
                    ptMaxTrackSize: POINT { x: 10000, y: 10000 },
                    ..Default::default()
                });
                return LRESULT(0);
            }

            wm::WM_SIZE => {
                let new_width = (lparam.0 & 0xffff) as u32;
                let new_height = ((lparam.0 >> 16) & 0xffff) as u32;
                debug!("WM_SIZE: {} x {}", new_width, new_height);

                if let Some(sb) = form.status_bar.take() {
                    form.status_bar.set(Some(sb.clone()));
                    SendMessageW(sb.handle(), WM_SIZE, None, None);
                }

                form.invalidate_layout();
                form.ensure_layout_valid();

                // TODO: is this now redundant, due to layout?
                if let Some(ref mdi_client) = form.mdi_client {
                    debug!("setting MDI client size to {} x {}", new_width, new_height);
                    _ = SetWindowPos(
                        mdi_client.control.handle(),
                        None,
                        0,
                        0,
                        new_width as i32,
                        new_height as i32,
                        SWP_NOZORDER,
                    );
                }

                // return 0;
            }

            wm::WM_COMMAND => {
                // https://docs.microsoft.com/en-us/windows/win32/menurc/wm-command

                let control = ControlId(wparam_loword(wparam));
                let command = Command(wparam_hiword(wparam) as u32);

                match command.0 {
                    wm::BN_CLICKED => {
                        debug!("BN_CLICKED");
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ButtonClicked,
                        });
                    }

                    wm::EN_CHANGE => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::EditChange,
                        });
                    }

                    wm::BN_SETFOCUS | wm::EN_SETFOCUS => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::SetFocus,
                        });
                    }

                    wm::BN_KILLFOCUS | wm::EN_KILLFOCUS => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::LostFocus,
                        });
                    }

                    _ => {
                        debug!("unrecognized WM_COMMAND code: {:#4x}", command.0);
                    }
                }

                if let Some(handler) = form.command_handler.get() {
                    handler(control, command);
                } else {
                    debug!("WM_COMMAND: no handler is installed");
                }
            }

            // WM_NOTIFY is used by most of the Common Controls to communicate
            // with the app.
            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-notify
            wm::WM_NOTIFY => {
                let nmhdr_ptr: *mut NMHDR = lparam.0 as *mut NMHDR;
                let hwnd_from: HWND = (*nmhdr_ptr).hwndFrom;
                let notify_code = (*nmhdr_ptr).code;
                // let notify = Notify::from_nmhdr(nmhdr_ptr);

                let control: ControlId = ControlId(wparam.0 as u16);

                use windows::Win32::UI::Controls as controls;

                // For some notifications, we need to handle the notification directly.
                match notify_code {
                    TCN_SELCHANGE => {
                        let tab_controls = form.tab_controls.borrow();
                        for weak_tab_control in tab_controls.iter() {
                            if let Some(tab_control) = weak_tab_control.upgrade() {
                                // TODO: check that this is the right tab control
                                tab_control.sync_visible();
                            }
                        }
                    }

                    // Used for ListView, TreeView
                    // https://learn.microsoft.com/en-us/windows/win32/controls/nm-click-list-view
                    controls::NM_CLICK => {
                        let details: &NMITEMACTIVATE = &*(lparam.0 as *const NMITEMACTIVATE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ItemClick {
                                item: details.iItem,
                                subitem: details.iSubItem,
                            },
                        });
                    }

                    // Used for ListView, TreeView
                    // https://learn.microsoft.com/en-us/windows/win32/controls/nm-dblclk-list-view
                    controls::NM_DBLCLK => {
                        let details: &NMITEMACTIVATE = &*(lparam.0 as *const NMITEMACTIVATE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ItemDoubleClick {
                                item: details.iItem,
                                subitem: details.iSubItem,
                            },
                        });
                    }

                    controls::LVN_COLUMNCLICK => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListColumnClick,
                        });
                    }

                    controls::LVN_ITEMCHANGED => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListItemChanged,
                        });
                    }

                    controls::LVN_ITEMACTIVATE => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListItemActivate,
                        });
                    }

                    controls::NM_RETURN => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::Return,
                        });
                    }

                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-itemchanged
                    controls::TVN_ITEMCHANGED => {
                        let item_change: &NMTVITEMCHANGE = &*(lparam.0 as *const NMTVITEMCHANGE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemChanged,
                        });
                    }

                    // TreeView - TVN_ITEMEXPANDED
                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-itemexpanded
                    controls::TVN_ITEMEXPANDED => {
                        let item_change: &NMTREEVIEWW = &*(lparam.0 as *const NMTREEVIEWW);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemExpanded,
                        });
                    }

                    // TreeView - TVN_SELCHANGED
                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-selchanged
                    controls::TVN_SELCHANGED => {
                        let item_change: &NMTREEVIEWW = &*(lparam.0 as *const NMTREEVIEWW);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemSelectionChanged,
                        });
                    }

                    _ => {}
                }

                /*
                if let Some(handler) = state.notify_handler.get() {
                    handler(&notify);
                } else {
                    debug!("no WM_NOTIFY handler installed");
                }
                */

                return LRESULT(0);
            }

            // https://docs.microsoft.com/en-us/windows/win32/winmsg/wm-sizing
            wm::WM_SIZING => {
                let (min_width, min_height) = form.layout_min_size.get();
                let window_size: &mut RECT = &mut *(lparam.0 as *mut RECT);
                let height = window_size.bottom - window_size.top;

                // TODO: These adjustments are made to the non-client area,
                // not to the client area.
                let mut adjusted_rect = RECT {
                    top: 0,
                    left: 0,
                    right: min_width,
                    bottom: min_height,
                };

                let window_style = WINDOW_STYLE(GetWindowLongW(window, GWL_STYLE) as u32);

                _ = AdjustWindowRect(&mut adjusted_rect, window_style, false);
                let min_width = adjusted_rect.right - adjusted_rect.left;
                let min_height = adjusted_rect.bottom - adjusted_rect.top;

                // If the width is too small, resist!
                let width = window_size.right - window_size.left;
                if width < min_width {
                    match wparam.0 as u32 {
                        WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT => {
                            window_size.right = window_size.left + min_width;
                        }
                        WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT => {
                            window_size.left = window_size.right - min_width;
                        }
                        _ => {}
                    }
                }

                // If the height is too small, resist!
                if height < min_height {
                    window_size.bottom = window_size.top + min_height;
                    match wparam.0 as u32 {
                        WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT => {
                            window_size.top = window_size.bottom - min_height;
                        }
                        WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT => {
                            window_size.bottom = window_size.top + min_height;
                        }
                        _ => {}
                    }
                }
            }

            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-ctlcolorstatic
            wm::WM_CTLCOLORSTATIC => {
                let hdc = HDC(wparam.0 as _);
                let brush = form.background_brush.borrow();
                if let Some(brush) = brush.as_ref() {
                    let hbrush = brush.handle();
                    SelectObject(hdc, HGDIOBJ(hbrush.0));
                    SetBkColor(hdc, COLORREF(form.background_color.get().as_u32()));
                    return LRESULT(hbrush.0 as _);
                }

                return LRESULT(0);
            }

            wm::WM_ERASEBKGND => {
                let mut client_rect: RECT = zeroed();
                _ = GetClientRect(window, &mut client_rect);
                let hdc = HDC(wparam.0 as _);
                let brush = form.background_brush.borrow();
                if let Some(brush) = brush.as_ref() {
                    let hbrush = brush.handle();
                    _ = FillRect(hdc, &client_rect, hbrush);
                    return LRESULT(hbrush.0 as _);
                }
            }

            // MDI frame events
            wm::WM_CHILDACTIVATE => {
                debug!("WM_CHILDACTIVATE");
            }

            wm::WM_MDIACTIVATE => {
                debug!("WM_MDIACTIVATE");
            }

            _ => {
                // allow default to run
            }
        }

        match form.mdi_mode {
            MdiMode::Child => DefMDIChildProcW(window, message, wparam, lparam),
            MdiMode::None => DefWindowProcW(window, message, wparam, lparam),
            MdiMode::Frame => DefFrameProcW(window, None, message, wparam, lparam),
        }
    }
}

#[inline(always)]
fn wparam_loword(wp: WPARAM) -> u16 {
    wp.0 as u16
}

#[allow(dead_code)]
#[inline(always)]
fn wparam_hiword(wp: WPARAM) -> u16 {
    (wp.0 >> 16) as u16
}

impl Drop for FormState {
    fn drop(&mut self) {
        // nothing right now
    }
}
