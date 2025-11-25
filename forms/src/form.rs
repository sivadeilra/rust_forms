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
mod wndproc;

pub use builder::*;

/// A top-level window.
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

impl Drop for Form {
    fn drop(&mut self) {
        // Break cycles inside form state
    }
}

impl Drop for FormState {
    fn drop(&mut self) {
        // nothing right now
    }
}
