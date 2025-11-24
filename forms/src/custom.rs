use super::*;
use crate::gdi::dc::Dc;
use core::mem::MaybeUninit;
use std::sync::Once;
use windows::core::w;

pub struct CustomControl<Inner>
where
    Inner: CustomInner,
{
    /// We use a box so that the location of the state is stable, so that the wndproc can
    /// dereference it.
    state: Box<State<Inner>>,
}

/// This is what the window GWLP_USERDATA points to.
#[repr(C)]
struct PolymorphicStateHeader {
    wndproc: unsafe fn(
        polymorphic_state: *mut PolymorphicStateHeader,
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT,
}

#[repr(C)]
struct State<Inner> {
    header: PolymorphicStateHeader,
    inner: Inner,
    control: Rc<ControlState>,
}

impl<Inner> std::ops::Deref for CustomControl<Inner>
where
    Inner: CustomInner + 'static,
{
    type Target = Rc<ControlState>;

    fn deref(&self) -> &Rc<ControlState> {
        &self.state.control
    }
}

impl<Inner> CustomControl<Inner>
where
    Inner: CustomInner + 'static,
{
    pub fn new(parent: &Rc<ControlState>, inner: Inner) -> Self {
        let atom = register_class_lazy();

        let ex_style = Default::default();
        let style = WS_VISIBLE | WS_CLIPSIBLINGS | WS_CHILD | WS_TABSTOP;

        let parent = Rc::clone(parent);

        unsafe {
            let hwnd = CreateWindowExW(
                ex_style,
                PCWSTR(atom as *const u16),
                w!(""),
                style,
                0,   // x
                0,   // y
                400, // width
                400, // height
                Some(parent.handle()),
                None, // hmenu
                None, // instance
                None, // Some(bouncer_ptr as *const _), // lpparam
            )
            .unwrap();

            debug!("created custom control");

            let control = ControlState::new(hwnd, Some(parent));

            let boxed_state: Box<State<Inner>> = Box::new(State {
                header: PolymorphicStateHeader {
                    wndproc: custom_wndproc_generic::<Inner>,
                },
                inner,
                control,
            });

            let state_ref: &State<Inner> = &*boxed_state;
            let state_ptr: *const State<Inner> = state_ref;

            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), state_ptr as isize);

            Self { state: boxed_state }
        }
    }
}

pub trait CustomInner: Sized {
    fn paint(&self, control: &ControlState, dc: &Dc, rect: &Rect) {}
    fn mouse_move(&self, control: &ControlState, pt: POINT) {}
    fn mouse_leave(&self, control: &ControlState) {}
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
        class_ex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
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
                                                                 // let bouncer: *const Bouncer = create_params as *const Bouncer;
                                                                 // debug!("custom_wndproc: WM_CREATE, bouncer: {bouncer:?}");
                                                                 // SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), bouncer as isize);
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }

        _ => {}
    }

    let polymorphic_state_header: *mut PolymorphicStateHeader =
        GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as _;
    if polymorphic_state_header.is_null() {
        // debug!("custom_wndproc: message 0x{message:04x} - no bouncer");
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }

    let inner_wndproc = (*polymorphic_state_header).wndproc;
    inner_wndproc(polymorphic_state_header, hwnd, message, wparam, lparam)
}

unsafe fn custom_wndproc_generic<Inner: CustomInner>(
    polymorphic_state: *mut PolymorphicStateHeader,
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let state_ptr = polymorphic_state as *mut State<Inner>;

    let state = &*state_ptr;

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

                state
                    .inner
                    .paint(&state.control, &dc, &rect_to_rectl(&paint.rcPaint));

                _ = EndPaint(hwnd, &paint);
            }

            WM_MOUSEMOVE => {
                let x = get_x_lparam(lparam) as i32;
                let y = get_y_lparam(lparam) as i32;
                state.inner.mouse_move(&state.control, POINT { x, y });
            }

            WM_MOUSELEAVE => {
                state.inner.mouse_leave(&state.control);
            }

            _ => {}
        }

        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}
