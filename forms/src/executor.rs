//! Async executor for apps that use window message pumps.
//!
//! When a new task is added to the executor, we allocate a `Task<F>` for it,
//! where `F` is the type that implements `Future`. The `Task<F>` type contains
//! enough memory to store the per-task state relevant to the executor
//! (including the `Waker` implementation) and `Future`-implementing object.
//!
//! The `Task<F>` object has a header, whose type and structure does not vary
//! per `F`. The `Waker` implementation points to this header. When the
//! `Waker` runs, it inspects the header, locks the executor's run queue,
//! inserts the task into the run queue, and schedules the executor to run.
//!
//! The `Waker` may be used on any thread; it is `Send` and `Sync`, and `Clone`.
//! Because a waker can be cloned, it can also be used more than once, so the
//! implementation must be resilient against redundant wake requests.
//!
//! TODO: This code works, but does not permit any real integration with the
//! `async` ecosystem (mainly centered on `tokio`). I need to understand better
//! how `tokio` schedules its work and how it interacts with its own I/O
//! wakers.

use super::*;
use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[derive(Clone)]
pub struct AsyncExecutor {
    state: Arc<ExecutorState>,
}

struct ExecutorState {
    inner: Mutex<ExecutorInnerState>,
    max_polls_per_cycle: AtomicU32,
    hwnd: HWND,
    #[cfg(feature = "tokio")]
    tokio_runtime: tokio::runtime::Runtime,
}

struct ExecutorInnerState {
    /// Tasks in this list are ready to be polled.
    runnable: VecDeque<Arc<dyn TaskTrait>>,

    // Set to true when we call PostMessage.
    is_scheduled: bool,
}

trait TaskTrait {
    fn poll(&self) -> Poll<()>;
}

struct TaskHeader {
    in_run_queue: AtomicBool,

    // Pointer to the executor.
    executor: std::sync::Weak<ExecutorState>,

    self_dyn: UnsafeCell<MaybeUninit<*const dyn TaskTrait>>,
}

#[repr(C)] // want linear layout
struct Task<F: Future> {
    header: TaskHeader,
    description: Cow<'static, str>,
    future: UnsafeCell<F>,
}

impl Default for AsyncExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncExecutor {
    pub fn new() -> AsyncExecutor {
        unsafe {
            let tokio_runtime;

            #[cfg(feature = "tokio")]
            {
                tokio_runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .build()
                    .unwrap();
            }
            #[cfg(not(feature = "tokio"))]
            {
                tokio_runtime = ();
                let _ = tokio_runtime;
            }

            // Create the messaging window.
            let window_class_atom = register_class_lazy();
            let instance = get_instance();
            let window_name: [u16; 2] = [0; 2];
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR::from_raw(window_class_atom as usize as *const u16),
                PCWSTR::from_raw(window_name.as_ptr()), // window name
                WINDOW_STYLE(0),                        // style
                0,                                      // x
                0,                                      // y
                1,                                      // width
                1,                                      // height
                HWND_MESSAGE,
                None,
                instance,
                None, // state_ptr.as_ptr() as *mut c_void,
            );
            if hwnd.0 == 0 {
                panic!("Failed to create messaging window for AsyncExecutor");
            }

            let state: Arc<ExecutorState> = Arc::new(ExecutorState {
                inner: Mutex::new(ExecutorInnerState {
                    runnable: Default::default(),
                    is_scheduled: false,
                }),
                max_polls_per_cycle: AtomicU32::new(32),
                hwnd,
                #[cfg(feature = "tokio")]
                tokio_runtime,
            });
            let state_ptr: *const ExecutorState = &*state;

            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), state_ptr as isize);

            // The alloc and window were successfully created.
            // Return a strong reference to the executor.
            AsyncExecutor { state }
        }
    }

    pub fn spawn<F>(&self, description: Cow<'static, str>, future: F)
    where
        F: Future<Output = ()> + 'static,
    {
        let executor_weak = Arc::downgrade(&self.state);

        let task: Arc<Task<F>> = Arc::new(Task::<F> {
            description,
            future: UnsafeCell::new(future),
            header: TaskHeader {
                executor: executor_weak,
                in_run_queue: AtomicBool::new(true),
                self_dyn: unsafe { core::mem::zeroed() },
            },
        });

        unsafe {
            // Set up what is essentially a vtable pointer.
            task.header.self_dyn.get().write(MaybeUninit::new(&*task));
        }

        {
            let mut inner = self.state.inner.lock().unwrap();
            inner.runnable.push_back(task);
        }

        self.state.schedule();
    }
}

impl Drop for AsyncExecutor {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.state.hwnd);
            // TODO: I do not know what the behavior is if we call DestroyWindow
            // on a window that has a message posted to it (PostMessage).
            // I am going to assume for now that the posted message remains in
            // the queue, and will be delivered.
        }
    }
}

impl ExecutorState {
    fn schedule(&self) {
        let mut inner = self.inner.lock().unwrap();
        let need_schedule = !inner.is_scheduled;
        if need_schedule {
            inner.is_scheduled = true;
        }
        drop(inner);

        if need_schedule {
            self.schedule_always();
        }
    }

    fn schedule_always(&self) {
        unsafe {
            if PostMessageW(self.hwnd, EXECUTOR_WM_POLL, WPARAM(0), LPARAM(0)).into() {
                trace!("posted EXECUTOR_WM_POLL");
            } else {
                // PostMessage failed. Untrack the post_is_posted state.
                warn!("PostMessageW failed: {}", GetLastError().0);
                // state.as_mut().poll_is_posted = false;
            }
        }
    }

    fn poll_all(&self) {
        let mut num_polls: u32 = 0;
        let mut need_reschedule = false;

        let tokio_runtime_guard;
        #[cfg(feature = "tokio")]
        {
            tokio_runtime_guard = self.tokio_runtime.enter();
        }
        #[cfg(not(feature = "tokio"))]
        {
            tokio_runtime_guard = ();
            let _ = tokio_runtime_guard;
        }

        loop {
            let task;
            {
                let mut inner = self.inner.lock().unwrap();

                if inner.runnable.is_empty() {
                    trace!("poll_all: runnable list is empty");
                    break;
                }

                // We want to avoid doing nothing but running async work,
                // if the async tasks indicate that they are still runnable.
                // We want to run a mix of message dispatch and async dispatch.
                // We can't guarantee any kind of fairness or queuing policy,
                // but we can do better than total starvation of non-async work.
                let max_polls_per_cycle = self.max_polls_per_cycle.load(SeqCst);
                if num_polls >= max_polls_per_cycle {
                    trace!("poll_all: reached maximum polls per cycle ({})", num_polls);
                    need_reschedule = true;
                    break;
                }

                if let Some(t) = inner.runnable.pop_front() {
                    task = t;
                } else {
                    break;
                }
            };

            // This is the point where we execute a task (poll it). That may
            // execute arbitrary code, so we have to be prepared for
            // certain forms of re-entrancy, here. For this reason, this
            // method takes &self, not &mut self, and we are careful not to
            // hold any dynamic borrows when we call poll().

            match task.poll() {
                Poll::Ready(()) => {
                    // This task has completed. Free the task.
                    trace!("task has completed");
                    drop(task);
                }

                Poll::Pending => {
                    // This task needs to do some work. It will wake us up
                    // when it needs to do more work, using the waker.
                    // We don't need to do anything more with it, here.
                    trace!("task returned Poll::Pending");
                }
            }

            num_polls += 1;
        }

        // It is possible for self.runnable to still have items in it. If so,
        // we re-schedule the executor.
        if need_reschedule {
            self.schedule();
        }
    }
}

static WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(waker_clone, waker_wake, waker_wake_by_ref, waker_drop);

unsafe fn unpack_waker_ptr(waker_ptr: *const ()) -> (*const TaskHeader, *const dyn TaskTrait) {
    let header = waker_ptr as *const TaskHeader;
    let self_dyn: *const dyn TaskTrait = (*(*header).self_dyn.get()).assume_init();
    (header, self_dyn)
}

unsafe fn waker_clone(waker_ptr: *const ()) -> RawWaker {
    let (_header, self_dyn) = unpack_waker_ptr(waker_ptr);

    Arc::increment_strong_count(self_dyn);
    trace!("waker_clone");
    RawWaker::new(waker_ptr, &WAKER_VTABLE)
}

unsafe fn waker_wake(waker_ptr: *const ()) {
    trace!("waker_wake");
    waker_wake_by_ref(waker_ptr);
    waker_drop(waker_ptr);
}

unsafe fn waker_wake_by_ref(waker_ptr: *const ()) {
    trace!("waker_wake_by_ref");

    let (header, self_dyn) = unpack_waker_ptr(waker_ptr);

    if let Some(executor) = (*header).executor.upgrade() {
        let mut need_scheduled = false;
        let mut inner = executor.inner.lock().unwrap();
        let is_task_scheduled = (*header).in_run_queue.load(SeqCst);
        if is_task_scheduled {
            trace!("- this task is already scheduled");
        } else {
            trace!("- placing task into run queue");
            Arc::increment_strong_count(self_dyn);
            need_scheduled = inner.runnable.is_empty();
            inner.runnable.push_back(Arc::from_raw(self_dyn));
        }
        drop(inner); // unlock

        if need_scheduled {
            executor.schedule_always();
        }
    } else {
        error!("cannot wake up, because executor is gone!");
    }
}

unsafe fn waker_drop(waker_ptr: *const ()) {
    trace!("waker_drop");

    let (_header, self_dyn) = unpack_waker_ptr(waker_ptr);

    // Drop the reference.
    drop(Arc::from_raw(self_dyn));
}

impl<F: Future<Output = ()>> TaskTrait for Task<F> {
    fn poll(&self) -> Poll<()> {
        self.header.in_run_queue.store(false, SeqCst);

        trace!("polling {:?}", self.description);

        unsafe {
            let waker: ManuallyDrop<Waker> = ManuallyDrop::new(Waker::from_raw(RawWaker::new(
                self as *const Self as *const (),
                &WAKER_VTABLE,
            )));

            let mut cx: Context = Context::from_waker(&waker);
            Pin::new_unchecked(&mut *self.future.get()).poll(&mut cx)
        }
    }
}

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut EXECUTOR_CLASS_ATOM: ATOM = 0;

const EXECUTOR_CLASS_NAME: &str = "RustForms_AsyncExecutor";
const EXECUTOR_WM_POLL: u32 = WM_USER;

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(EXECUTOR_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_mut_ptr());
        class_ex.style = WNDCLASS_STYLES(0);
        class_ex.hbrBackground = HBRUSH(0);
        class_ex.lpfnWndProc = Some(executor_wndproc);
        class_ex.hCursor = HCURSOR(0);
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        EXECUTOR_CLASS_ATOM = atom;
    });

    unsafe { EXECUTOR_CLASS_ATOM }
}

extern "system" fn executor_wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_CREATE => {
                trace!("executor_wndproc: WM_CREATE");
                return LRESULT(1);
            }

            _ => {}
        }

        let state_ptr: isize = GetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0));
        if state_ptr == 0 {
            trace!("executor_wndproc: lparam is null, msg {:04x}", message);
            return DefWindowProcW(window, message, wparam, lparam);
        }

        let state: *const ExecutorState = state_ptr as *const ExecutorState;

        match message {
            EXECUTOR_WM_POLL => {
                trace!("received EXECUTOR_WM_POLL");
                (*state).poll_all();
                return LRESULT(0);
            }

            WM_DESTROY => {
                return LRESULT(0);
            }

            _ => {
                // allow default to run
            }
        }

        DefWindowProcW(window, message, wparam, lparam)
    }
}
