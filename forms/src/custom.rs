use super::*;
use crate::gdi::dc::Dc;
use core::mem::MaybeUninit;
use std::sync::Once;
use windows::w;

pub struct CustomControl<Inner>
where
    Inner: CustomInner,
{
    control: ControlState,
    inner: Inner,

    bouncer: MaybeUninit<Bouncer>,
}

trait Bounced {
    fn wndproc(&self, hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

#[repr(C)]
struct Bouncer {
    ptr: *const dyn Bounced,
}

impl<Inner> std::ops::Deref for CustomControl<Inner>
where
    Inner: CustomInner + 'static,
{
    type Target = ControlState;

    fn deref(&self) -> &ControlState {
        &self.control
    }
}

impl<Inner> CustomControl<Inner>
where
    Inner: CustomInner + 'static,
{
    pub fn new(parent: &ControlState, inner: Inner) -> Rc<Self> {
        let atom = register_class_lazy();

        let ex_style = Default::default();
        let style = WS_VISIBLE | WS_CLIPSIBLINGS | WS_CHILD | WS_TABSTOP;

        unsafe {
            let mut me = Rc::new(Self {
                control: ControlState::new(HWND(0)),
                inner,
                bouncer: MaybeUninit::zeroed(),
            });
            let only_me = Rc::get_mut(&mut me).unwrap();
            let the_ptr: *const dyn Bounced = only_me as &dyn Bounced;
            only_me.bouncer.write(Bouncer { ptr: the_ptr }); // make a self-referential structure
            let bouncer_ptr = only_me.bouncer.as_mut_ptr();

            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR(atom as *const u16),
                w!(""),
                style,
                0,   // x
                0,   // y
                400, // width
                400, // height
                parent.handle(),
                HMENU(0),                      // hmenu
                None,                          // instance
                Some(bouncer_ptr as *const _), // lpparam
            );

            if hwnd.0 == 0 {
                panic!("Failed to create custom window");
            }

            debug!("created custom control");

            let only_me = Rc::get_mut(&mut me).unwrap();
            only_me.control.hwnd = hwnd;

            me
        }
    }
}

pub trait CustomInner: Sized {
    fn paint(&self, control: &CustomControl<Self>, dc: &Dc, rect: &Rect) {}
    fn mouse_move(&self, control: &CustomControl<Self>, pt: POINT) {}
    fn mouse_leave(&self, control: &CustomControl<Self>) {}
}

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut CLASS_ATOM: ATOM = 0;

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let instance = get_instance();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = w!("rust_forms.custom");
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_WINDOW.0 + 1) as _);
        class_ex.lpfnWndProc = Some(custom_wndproc);
        class_ex.hCursor = LoadCursorW(HMODULE(0), IDC_ARROW).unwrap();
        class_ex.cbWndExtra = (size_of::<*mut c_void>() * 2) as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        CLASS_ATOM = atom;
    });

    unsafe { CLASS_ATOM }
}

unsafe extern "system" fn custom_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let create_struct = lparam.0 as *const CREATESTRUCTW;
            let create_params = (*create_struct).lpCreateParams; // <-- this points to Bouncer
            let bouncer: *const Bouncer = create_params as *const Bouncer;
            debug!("custom_wndproc: WM_CREATE, bouncer: {bouncer:?}");
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), bouncer as isize);
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }

        _ => {}
    }

    let bouncer_ptr: isize = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0));
    if bouncer_ptr == 0 {
        // debug!("custom_wndproc: message 0x{message:04x} - no bouncer");
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }

    // debug!("custom_wndproc: message 0x{message:04x}");
    let bouncer: *const Bouncer = bouncer_ptr as *const Bouncer;
    let dyn_ptr = (*bouncer).ptr;
    (*dyn_ptr).wndproc(hwnd, message, wparam, lparam)
}

impl<Inner> Bounced for CustomControl<Inner>
where
    Inner: CustomInner,
{
    fn wndproc(&self, hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging as wm;

            match message {
                /*
                WM_COMMAND | WM_NOTIFY => {
                    // Forward WM_COMMAND and WM_NOTIFY up the window tree.
                    let parent_hwnd = GetParent(hwnd);
                    return SendMessageW(parent_hwnd, message, wparam, lparam);
                }
                */
                WM_PAINT => {
                    let mut paint: PAINTSTRUCT = core::mem::zeroed();
                    BeginPaint(hwnd, &mut paint);

                    let dc = Dc { hdc: paint.hdc };

                    self.inner.paint(self, &dc, &rect_to_rectl(&paint.rcPaint));

                    EndPaint(hwnd, &paint);
                }

                WM_MOUSEMOVE => {
                    let x = get_x_lparam(lparam) as i32;
                    let y = get_y_lparam(lparam) as i32;
                    self.inner.mouse_move(self, POINT { x, y });
                }

                WM_MOUSELEAVE => {
                    self.inner.mouse_leave(self);
                }

                _ => {}
            }

            DefWindowProcW(hwnd, message, wparam, lparam)
        }
    }
}
