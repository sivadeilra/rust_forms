use super::*;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use log::debug;
use std::sync::Once;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

mod builder;

pub use builder::*;

/// A top-level window.
pub struct Form {
    stuck: StuckToThread,

    co_initialized: bool,

    pub(crate) handle: Cell<HWND>,
    quit_on_close: Option<i32>,

    is_layout_valid: Cell<bool>,
    layout_min_size: Cell<(i32, i32)>,

    pub(crate) layout: RefCell<Option<Layout>>,

    /// Used for event routing.
    pub(crate) notify_handlers: RefCell<HashMap<HWND, NotifyHandler>>,
    pub(crate) event_handlers: RefCell<HashMap<HWND, Rc<dyn MessageHandlerTrait>>>,
    pub(crate) default_edit_font: Cell<Option<Rc<Font>>>,
    pub(crate) default_button_font: Cell<Option<Rc<Font>>>,
    pub(crate) background_brush: Cell<Option<Brush>>,
    pub(crate) background_color: Cell<ColorRef>,

    status_bar: Cell<Option<Rc<StatusBar>>>,
}

assert_not_impl_any!(Form: Send, Sync);

pub(crate) trait MessageHandlerTrait: 'static {
    fn wm_command(&self, control_id: u16, notify_code: u16) -> LRESULT {
        let _ = (control_id, notify_code);
        0
    }

    fn handle_message(&self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        let _ = (msg, wparam, lparam);
        0
    }
}

pub(crate) struct NotifyHandler {
    pub(crate) handler: Rc<dyn NotifyHandlerTrait>,
}

pub(crate) trait NotifyHandlerTrait {
    unsafe fn wm_notify(&self, control_id: WPARAM, nmhdr: *mut NMHDR) -> NotifyResult;
}

pub(crate) enum NotifyResult {
    #[allow(dead_code)]
    Consumed(LRESULT),
    NotConsumed,
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
            ShowWindow(self.handle(), SW_SHOW);
        }
    }

    pub fn set_title(&self, text: &str) {
        self.stuck.check();
        set_window_text(self.handle.get(), text);
    }

    pub fn set_default_edit_font(&self, font: Option<Rc<Font>>) {
        self.stuck.check();
        self.default_edit_font.set(font);
    }

    pub fn get_default_edit_font(&self) -> Option<Rc<Font>> {
        self.stuck.check();
        clone_cell_opt_rc(&self.default_edit_font)
    }

    pub fn set_default_button_font(&self, font: Option<Rc<Font>>) {
        self.stuck.check();
        self.default_button_font.set(font);
    }

    pub fn get_default_button_font(&self) -> Option<Rc<Font>> {
        self.stuck.check();
        clone_cell_opt_rc(&self.default_button_font)
    }

    pub fn set_menu(&self, menu: Option<Menu>) {
        self.stuck.check();
        unsafe {
            if let Some(menu) = menu {
                let hmenu = menu.extract();
                if !SetMenu(self.handle.get(), hmenu).as_bool() {
                    warn!("failed to set menu for form: {}", GetLastError());
                }
            } else {
                if SetMenu(self.handle.get(), 0).as_bool() {
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
                SendMessageW(sb.handle(), WM_SIZE, 0, 0);
                let mut sb_rect: RECT = zeroed();
                GetClientRect(sb.handle(), &mut sb_rect);
                sb_height = sb_rect.bottom - sb_rect.top;
            }

            let mut client_rect: RECT = zeroed();
            if GetClientRect(self.handle.get(), &mut client_rect).into() {
                debug!(
                    "running layout, rect: {},{} - {},{}",
                    client_rect.left, client_rect.top, client_rect.right, client_rect.bottom
                );

                let layout_opt = self.layout.borrow();
                if let Some(layout) = &*layout_opt {
                    let min_size = layout.get_min_size();
                    self.layout_min_size.set(min_size);

                    let mut layout_height = client_rect.bottom - client_rect.top;
                    if layout_height >= sb_height {
                        layout_height -= sb_height;
                    }

                    struct DeferredLayoutPlacer {
                        op: DeferWindowPosOp,
                    }

                    impl LayoutPlacer for DeferredLayoutPlacer {
                        fn place_control(
                            &mut self,
                            control: &ControlState,
                            x: i32,
                            y: i32,
                            width: i32,
                            height: i32,
                        ) {
                            self.op
                                .defer(control.handle(), 0, x, y, width, height, SWP_NOZORDER);
                        }
                    }

                    let mut deferred_placer = DeferredLayoutPlacer {
                        op: DeferWindowPosOp::begin(10).unwrap(),
                    };

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
        self.stuck.check();
        self.show_window();
        self.event_loop();
    }

    fn event_loop(&self) {
        unsafe {
            loop {
                let mut msg: MSG = zeroed();
                let ret = GetMessageW(&mut msg, 0, 0, 0).0;
                if ret < 0 {
                    debug!("event loop: GetMessageW returned {}, quitting", ret);
                    break;
                }

                if msg.message == WM_QUIT {
                    debug!("found WM_QUIT, quitting");
                    break;
                }

                if IsDialogMessageW(self.handle(), &msg).into() {
                    continue;
                }

                TranslateMessage(&mut msg);
                DispatchMessageW(&mut msg);
            }
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
        class_ex.lpszClassName = PWSTR(class_name_wstr.as_mut_ptr());
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = (COLOR_WINDOW + 1) as _;
        class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = LoadCursorW(0isize, IDC_ARROW);
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
            WM_CREATE => {
                let create_struct: *mut CREATESTRUCTW = lparam as *mut CREATESTRUCTW;
                assert!(!create_struct.is_null());

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                debug!(
                    "WM_CREATE, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                SetWindowLongPtrW(window, 0, create_params as isize);
                return 1;
            }

            _ => {}
        }

        let state_ptr: isize = GetWindowLongPtrW(window, 0);
        if state_ptr == 0 {
            debug!("form_wndproc: lparam is null, msg {:04x}", message);
            return DefWindowProcW(window, message, wparam, lparam);
        }

        let state: &Form = &*(state_ptr as *const Form);

        match message {
            /*
            WM_PAINT => {
                println!("WM_PAINT");
                // ValidateRect(window, std::ptr::null());
                0
            }
            */
            WM_CLOSE => {
                if let Some(exit_code) = state.quit_on_close {
                    debug!("WM_CLOSE: posting quit message");
                    post_quit_message(exit_code);
                } else {
                    debug!("WM_CLOSE: not posting quit message");
                }
            }

            WM_DESTROY => {
                debug!("WM_DESTROY");
                return 0;
            }

            WM_SIZE => {
                let new_width = (lparam & 0xffff) as u32;
                let new_height = ((lparam >> 16) & 0xffff) as u32;
                trace!("WM_SIZE: {} x {}", new_width, new_height);

                if let Some(sb) = state.status_bar.take() {
                    state.status_bar.set(Some(sb.clone()));
                    SendMessageW(sb.handle(), WM_SIZE, 0, 0);
                }

                state.invalidate_layout();
                state.ensure_layout_valid();

                // return 0;
            }

            WM_COMMAND => {
                // https://docs.microsoft.com/en-us/windows/win32/menurc/wm-command

                if lparam != 0 {
                    // It's a child window handle.
                    let child_hwnd: HWND = lparam as HWND;

                    let event_handlers = state.event_handlers.borrow();
                    if let Some(handler) = event_handlers.get(&child_hwnd) {
                        debug!(
                            "WM_COMMAND: 0x{:x} hwnd 0x{:x} - found handler",
                            wparam, lparam
                        );

                        let h = Rc::clone(handler);
                        drop(event_handlers); // drop the dynamic borrow

                        let notify_code = (wparam >> 16) as u16;
                        let control_id = wparam as u16;

                        return h.wm_command(control_id, notify_code);
                    } else {
                        debug!(
                            "WM_COMMAND: 0x{:x} hwnd 0x{:x} - no handler found",
                            wparam, lparam
                        );
                        return 0;
                    }
                } else {
                    debug!("WM_COMMAND: 0x{:x}", wparam);
                    // return 0;
                }
            }

            // WM_NOTIFY is used by most of the Common Controls to communicate
            // with the app.
            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-notify
            WM_NOTIFY => {
                let nmhdr_ptr: *mut NMHDR = lparam as *mut NMHDR;
                let hwnd_from: HWND = (*nmhdr_ptr).hwndFrom;
                if false {
                    trace!(
                        "WM_NOTIFY: code 0x{:x} hwnd_from 0x{:x}",
                        (*nmhdr_ptr).code,
                        hwnd_from
                    );
                }
                // Look up the control by window handle.
                let notify_handlers = state.notify_handlers.borrow();
                if let Some(control) = notify_handlers.get(&hwnd_from) {
                    // Clone the Rc.
                    let cloned_control = control.handler.clone();
                    drop(notify_handlers); // drop dynamic borrow
                    match cloned_control.wm_notify(wparam, nmhdr_ptr) {
                        NotifyResult::NotConsumed => {}
                        NotifyResult::Consumed(result) => return result,
                    }
                } else {
                    debug!("WM_NOTIFY: received notification for unknown control window");
                    // return 0;
                }
            }

            // https://docs.microsoft.com/en-us/windows/win32/winmsg/wm-sizing
            WM_SIZING => {
                let (min_width, min_height) = state.layout_min_size.get();
                let window_size: &mut RECT = &mut *(lparam as *mut RECT);
                let height = window_size.bottom - window_size.top;

                // TODO: These adjustments are made to the non-client area,
                // not to the client area.
                let mut adjusted_rect = RECT {
                    top: 0,
                    left: 0,
                    right: min_width,
                    bottom: min_height,
                };

                let window_style = GetWindowLongW(window, GWL_STYLE) as u32;

                AdjustWindowRect(&mut adjusted_rect, window_style, false);
                let min_width = adjusted_rect.right - adjusted_rect.left;
                let min_height = adjusted_rect.bottom - adjusted_rect.top;

                // If the width is too small, resist!
                let width = window_size.right - window_size.left;
                if width < min_width {
                    match wparam as u32 {
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
                    match wparam as u32 {
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
            WM_CTLCOLORSTATIC => {
                let hdc = wparam as HDC;
                let brush_opt = state.background_brush.take();
                if let Some(brush) = brush_opt.as_ref() {
                    let hbrush = brush.handle();
                    SelectObject(hdc, hbrush);
                    state.background_brush.set(brush_opt);
                    SetBkColor(hdc, state.background_color.get().as_u32());
                    return hbrush;
                }

                return 0;
            }

            _ => {
                // allow default to run
            }
        }

        DefWindowProcW(window, message, wparam, lparam)
    }
}

impl Drop for Form {
    fn drop(&mut self) {
        self.stuck.check();

        unsafe {
            DestroyWindow(self.handle.get());

            if self.co_initialized {
                CoUninitialize();
            }
        }
    }
}
