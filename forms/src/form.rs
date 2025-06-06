use super::*;
use crate::msg::Msg;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use std::cell::OnceCell;
use std::sync::Once;
use tracing::debug;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::WindowsAndMessaging as wm;

mod builder;

pub use builder::*;

/// A top-level window.
pub struct Form {
    stuck: StuckToThread,

    co_initialized: bool,

    pub(crate) control: OnceCell<ControlState>,
    pub(crate) handle: Cell<HWND>,
    quit_on_close: Option<i32>,

    is_layout_valid: Cell<bool>,
    layout_min_size: Cell<(i32, i32)>,

    pub(crate) layout: RefCell<Option<Layout>>,
    pub(crate) style: Rc<Style>,
    pub(crate) background_brush: Cell<Option<Brush>>,
    pub(crate) background_color: Cell<ColorRef>,

    command_handler: OnceCell<Box<dyn Fn(ControlId, Command)>>,
    notify_handler: OnceCell<Box<dyn Fn(&Notify)>>,

    status_bar: Cell<Option<Rc<StatusBar>>>,

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
    type Target = ControlState;
    fn deref(&self) -> &Self::Target {
        self.control.get().unwrap()
    }
}

impl Form {
    pub fn builder<'a>() -> FormBuilder<'a> {
        FormBuilder::default()
    }

    pub(crate) fn handle(&self) -> HWND {
        self.stuck.check();
        self.handle.get()
    }

    pub fn show_window(&self) {
        self.stuck.check();
        self.ensure_layout_valid();
        unsafe {
            _ = ShowWindow(self.handle(), SW_SHOW);
        }
    }

    pub fn set_title(&self, text: &str) {
        self.stuck.check();
        set_window_text(self.handle.get(), text);
    }

    pub fn style(&self) -> &Style {
        &self.style
    }

    pub fn set_menu(&self, menu: Option<Menu>) {
        self.stuck.check();
        unsafe {
            if let Some(menu) = menu {
                let hmenu = menu.extract();
                if SetMenu(self.handle.get(), Some(hmenu)).is_err() {
                    warn!("failed to set menu for form: {:?}", GetLastError());
                }
            } else {
                if SetMenu(self.handle.get(), None).is_ok() {
                    trace!("cleared menu for form");
                } else {
                    warn!("failed to clear menu for form");
                }
            }
        }
    }

    pub fn create_status_bar(self: &Rc<Self>) -> Rc<StatusBar> {
        self.stuck.check();
        let sb = if let Some(sb) = self.status_bar.take() {
            sb
        } else {
            StatusBar::new(self)
        };
        self.status_bar.set(Some(sb.clone()));
        sb
    }

    pub fn get_status_bar(&self) -> Option<Rc<StatusBar>> {
        self.stuck.check();
        if let Some(sb) = self.status_bar.take() {
            self.status_bar.set(Some(sb.clone()));
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
        let result = self.command_handler.set(Box::new(handler));
        assert!(
            result.is_ok(),
            "cannot call command_handler() more than once"
        );
    }

    pub fn notify_handler<F>(&self, handler: F)
    where
        F: Fn(&Notify) + 'static,
    {
        let result = self.notify_handler.set(Box::new(handler));
        assert!(
            result.is_ok(),
            "cannot call notify_handler() more than once"
        );
    }
}

impl Form {
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
            if GetClientRect(self.handle.get(), &mut client_rect).is_ok() {
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

    pub fn set_layout(&self, layout: Layout) {
        self.stuck.check();
        let mut layout_borrow = self.layout.borrow_mut();
        *layout_borrow = Some(layout);
        drop(layout_borrow);

        self.invalidate_layout();
        self.ensure_layout_valid();
    }
}

impl Form {
    pub fn show_modal(&self) {
        self.show_modal_under(None)
    }

    pub fn show_modal_under(&self, parent: Option<&Form>) {
        self.stuck.check();

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

                debug!("wm 0x{:04x}", msg.message);

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

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut FORM_CLASS_ATOM: ATOM = 0;

const FORM_CLASS_NAME: &str = "RustForms_Form";

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
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
        FORM_CLASS_ATOM = atom;
    });

    unsafe { FORM_CLASS_ATOM }
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

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                debug!(
                    "WM_CREATE, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                SetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0), create_params as isize);
                return LRESULT(1);
            }

            _ => {}
        }

        let state_ptr: isize = GetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0));
        if state_ptr == 0 {
            debug!("form_wndproc: lparam is null, msg {:04x}", message);
            return DefWindowProcW(window, message, wparam, lparam);
        }

        let state: &Form = &*(state_ptr as *const Form);

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
                if let Some(exit_code) = state.quit_on_close {
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

            wm::WM_SIZE => {
                let new_width = (lparam.0 & 0xffff) as u32;
                let new_height = ((lparam.0 >> 16) & 0xffff) as u32;
                trace!("WM_SIZE: {} x {}", new_width, new_height);

                if let Some(sb) = state.status_bar.take() {
                    state.status_bar.set(Some(sb.clone()));
                    SendMessageW(sb.handle(), WM_SIZE, None, None);
                }

                state.invalidate_layout();
                state.ensure_layout_valid();

                // return 0;
            }

            wm::WM_COMMAND => {
                // https://docs.microsoft.com/en-us/windows/win32/menurc/wm-command

                let control = ControlId(wparam_loword(wparam));
                let command = Command(wparam_hiword(wparam) as u32);

                if let Some(handler) = state.command_handler.get() {
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
                let notify = Notify::from_nmhdr(nmhdr_ptr);

                // For some notifications, we need to handle the notification directly.
                match notify_code {
                    TCN_SELCHANGE => {
                        let tab_controls = state.tab_controls.borrow();
                        for weak_tab_control in tab_controls.iter() {
                            if let Some(tab_control) = weak_tab_control.upgrade() {
                                // TODO: check that this is the right tab control
                                tab_control.sync_visible();
                            }
                        }
                    }
                    _ => {}
                }

                if let Some(handler) = state.notify_handler.get() {
                    handler(&notify);
                } else {
                    debug!("no WM_NOTIFY handler installed");
                }
            }

            // https://docs.microsoft.com/en-us/windows/win32/winmsg/wm-sizing
            wm::WM_SIZING => {
                let (min_width, min_height) = state.layout_min_size.get();
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
                let brush_opt = state.background_brush.take();
                if let Some(brush) = brush_opt.as_ref() {
                    let hbrush = brush.handle();
                    SelectObject(hdc, HGDIOBJ(hbrush.0));
                    state.background_brush.set(brush_opt);
                    SetBkColor(hdc, COLORREF(state.background_color.get().as_u32()));
                    return LRESULT(hbrush.0 as _);
                }

                return LRESULT(0);
            }

            _ => {
                // allow default to run
            }
        }

        DefWindowProcW(window, message, wparam, lparam)
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

impl Drop for Form {
    fn drop(&mut self) {
        self.stuck.check();

        unsafe {
            _ = DestroyWindow(self.handle.get());

            if self.co_initialized {
                CoUninitialize();
            }
        }
    }
}
