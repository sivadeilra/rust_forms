use super::*;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;
use log::debug;
use std::sync::Once;

mod bg;
mod builder;
mod pipe;

pub use builder::*;
pub use pipe::Sender;

/// A top-level window.
///
/// The `Clone` implementation for this type clones a reference to the form,
/// not the contents of the form. This allows event handlers to capture state,
/// if needed.
#[derive(Clone)]
pub struct Form {
    pub(crate) state: Rc<FormState>,
}

assert_not_impl_any!(Form: Send, Sync);

/// This is the data that is pointed-to by the window user data field.
pub(crate) struct FormState {
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
    pub(crate) receivers: RefCell<Vec<Rc<dyn pipe::QueueReceiver>>>,
}

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
    unsafe fn wm_notify(&self, control_id: WPARAM, nmhdr: *mut NMHDR) -> LRESULT;
}

impl Form {
    pub fn builder<'a>() -> FormBuilder<'a> {
        FormBuilder::default()
    }

    pub(crate) fn handle(&self) -> HWND {
        self.state.handle.get()
    }

    pub fn show_window(&self) {
        self.state.ensure_layout_valid();
        unsafe {
            ShowWindow(self.handle(), SW_SHOW);
        }
    }

    pub fn set_title(&self, text: &str) {
        set_window_text(self.state.handle.get(), text);
    }

    pub fn set_default_edit_font(&self, font: Option<Rc<Font>>) {
        self.state.default_edit_font.set(font);
    }

    pub fn get_default_edit_font(&self) -> Option<Rc<Font>> {
        clone_cell_opt_rc(&self.state.default_edit_font)
    }

    pub fn set_default_button_font(&self, font: Option<Rc<Font>>) {
        self.state.default_button_font.set(font);
    }

    pub fn get_default_button_font(&self) -> Option<Rc<Font>> {
        clone_cell_opt_rc(&self.state.default_button_font)
    }

    pub fn set_menu(&self, menu: Option<Menu>) {
        unsafe {
            if let Some(menu) = menu {
                let hmenu = menu.extract();
                if SetMenu(self.state.handle.get(), hmenu).into() {
                    debug!("successfully set menu for form");
                } else {
                    warn!("failed to set menu for form: {}", GetLastError());
                }
            } else {
                if SetMenu(self.state.handle.get(), 0).into() {
                    trace!("cleared menu for form");
                } else {
                    warn!("failed to clear menu for form");
                }
            }
        }
    }

    pub fn set_layout(&self, layout: Layout) {
        self.state.set_layout(layout);
    }
}

const FORM_WM_BACKGROUND_COMPLETION: u32 = WM_USER + 0;
const FORM_WM_POLL_PIPE_RECEIVERS: u32 = WM_USER + 1;

impl FormState {
    pub(crate) fn invalidate_layout(&self) {
        self.is_layout_valid.set(false);
    }

    fn ensure_layout_valid(&self) {
        if self.is_layout_valid.get() {
            trace!("layout is already valid");
            return;
        }

        unsafe {
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

                    layout.place(
                        client_rect.left,
                        client_rect.top,
                        client_rect.right - client_rect.left,
                        client_rect.bottom - client_rect.top,
                    );
                }
                self.is_layout_valid.set(true);
            } else {
                warn!("failed to get client rect");
            }
        }
    }

    pub fn set_layout(&self, layout: Layout) {
        let mut layout_borrow = self.layout.borrow_mut();
        *layout_borrow = Some(layout);
        drop(layout_borrow);

        self.invalidate_layout();
        self.ensure_layout_valid();
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
        class_ex.hbrBackground = (COLOR_WINDOWFRAME + 1) as _;
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

        let state: &FormState = &*(state_ptr as *const FormState);

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

                state.invalidate_layout();
                state.ensure_layout_valid();

                return 0;
            }

            FORM_WM_BACKGROUND_COMPLETION => {
                trace!("FORM_WM_BACKGROUND_COMPLETION");
                Form::finish_background(lparam);
                return 0;
            }

            FORM_WM_POLL_PIPE_RECEIVERS => {
                trace!("FORM_WM_POLL_PIPE_RECEIVERS");
                state.poll_receivers();
                return 0;
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
                    return 0;
                }
            }

            // WM_NOTIFY is used by most of the Common Controls to communicate
            // with the app.
            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-notify
            WM_NOTIFY => {
                let nmhdr_ptr: *mut NMHDR = lparam as *mut NMHDR;
                let hwnd_from: HWND = (*nmhdr_ptr).hwndFrom;
                // Look up the control by window handle.
                let notify_handlers = state.notify_handlers.borrow();
                if let Some(control) = notify_handlers.get(&hwnd_from) {
                    // Clone the Rc.
                    let cloned_control = control.handler.clone();
                    drop(notify_handlers); // drop dynamic borrow
                    return cloned_control.wm_notify(wparam, nmhdr_ptr);
                } else {
                    // debug!("WM_NOTIFY: received notification for unknown control window");
                    return 0;
                }
            }

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

            _ => {
                // allow default to run
            }
        }

        DefWindowProcW(window, message, wparam, lparam)
    }
}
