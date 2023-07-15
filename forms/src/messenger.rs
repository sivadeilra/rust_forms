//! Allows a GUI thread to receive messages from other threads.
//!

use super::*;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::Once;
use std::sync::{Arc, Mutex};
use windows::Win32::System::Threading::{QueueUserWorkItem, WORKER_THREAD_FLAGS};

#[derive(Clone)]
pub struct Messenger {
    state: Rc<MessengerState>,
}

static_assertions::assert_not_impl_any!(Messenger: Send, Sync);

struct MessengerState {
    hwnd: HWND,
    receivers: RefCell<Vec<Rc<dyn QueueReceiver>>>,
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    run_on_main: VecDeque<Box<dyn QueueReceiver>>,
    hwnd: HWND,
}

impl Default for Messenger {
    fn default() -> Self {
        Self::new()
    }
}

impl Messenger {
    pub fn new() -> Messenger {
        unsafe {
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
                Some(&HWND_MESSAGE),
                None,
                instance,
                None,
            );
            if hwnd.0 == 0 {
                panic!("Failed to create messaging window for Messenger");
            }

            let state: Rc<MessengerState> = Rc::new(MessengerState {
                hwnd,
                receivers: Default::default(),
                shared_state: Arc::new(Mutex::new(SharedState {
                    run_on_main: VecDeque::new(),
                    hwnd,
                })),
            });
            let state_ptr: *const MessengerState = &*state;

            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), state_ptr as isize);

            // The alloc and window were successfully created.
            // Return a strong reference to the executor.
            Messenger { state }
        }
    }
}

static REGISTER_CLASS_ONCE: Once = Once::new();
static mut MESSENGER_CLASS_ATOM: ATOM = 0;

const EXECUTOR_CLASS_NAME: &str = "RustForms_Messenger";

const MESSENGER_WM_BACKGROUND_COMPLETION: u32 = WM_USER + 1;
const MESSENGER_WM_POLL_PIPE_RECEIVERS: u32 = WM_USER + 2;

fn register_class_lazy() -> ATOM {
    REGISTER_CLASS_ONCE.call_once(|| unsafe {
        let instance = get_instance();

        let class_name_wstr = U16CString::from_str(EXECUTOR_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_ptr());
        class_ex.style = WNDCLASS_STYLES(0);
        class_ex.hbrBackground = HBRUSH(0);
        class_ex.lpfnWndProc = Some(messenger_wndproc);
        class_ex.hCursor = HCURSOR(0);
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        MESSENGER_CLASS_ATOM = atom;
    });

    unsafe { MESSENGER_CLASS_ATOM }
}

extern "system" fn messenger_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_CREATE => {
                return LRESULT(1);
            }

            _ => {}
        }

        let state_ptr: isize = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0));
        if state_ptr == 0 {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }

        let state: &MessengerState = &*(state_ptr as *const MessengerState);

        match message {
            MESSENGER_WM_BACKGROUND_COMPLETION => {
                trace!("MESSENGER_WM_BACKGROUND_COMPLETION");
                /*
                let header = lparam as *mut BackgroundContextHeader;
                let completion_func = (*header).completion_func;
                completion_func(header);
                */
                return LRESULT(0);
            }

            MESSENGER_WM_POLL_PIPE_RECEIVERS => {
                trace!("MESSENGER_WM_POLL_PIPE_RECEIVERS");
                let mut i = 0;
                loop {
                    let receivers = state.receivers.borrow();
                    if i >= receivers.len() {
                        break;
                    }

                    let receiver_rc = Rc::clone(&receivers[i]);
                    drop(receivers); // drop dynamic borrow
                    i += 1;

                    // We drop the dynamic borrow of the receivers collection so that
                    // we can safely call into this app callback, without worrying about
                    // the app modifying the receiver set.
                    receiver_rc.run_on_main_thread();
                }

                loop {
                    let mut g = state.shared_state.lock().unwrap();
                    let item_opt = g.run_on_main.pop_front();
                    drop(g);

                    if let Some(item) = item_opt {
                        // It would be beneficial if we could call FnOnce() on
                        // a boxed closure, or method of a trait object that
                        // took `self`.
                        item.run_on_main_thread();
                        drop(item);
                    } else {
                        break;
                    }
                }

                return LRESULT(0);
            }

            _ => {}
        }

        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

impl Drop for MessengerState {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}

struct BackgroundContextHeader {
    // completion_func: unsafe fn(header: *mut BackgroundContextHeader),
}

#[repr(C)] // want linear layout
struct BackgroundContext<Worker, WorkOutput, Finisher> {
    header: BackgroundContextHeader,
    hwnd: HWND,
    worker: MaybeUninit<Worker>,
    output: UnsafeCell<MaybeUninit<std::thread::Result<WorkOutput>>>,
    finisher: UnsafeCell<MaybeUninit<Finisher>>,
    shared_state: Arc<Mutex<SharedState>>,
}

impl<Worker, WorkOutput, Finisher> QueueReceiver for BackgroundContext<Worker, WorkOutput, Finisher>
where
    Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
    WorkOutput: Send + 'static,
    Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
{
    fn run_on_main_thread(&self) {
        // In this state:
        // - 'worker' field is dead
        // - 'output' field is live
        // - 'finisher' field is live
        unsafe {
            let output = self.output.get().read().assume_init();
            let finisher = self.finisher.get().read().assume_init();
            finisher(output);
        }
    }
}

impl Messenger {
    /// Run some work on a background thread. When the work completes, run
    /// a function which completes the work, on the UI thread.
    ///
    /// This is a "one-shot" background request.
    pub fn run_background<Worker, WorkOutput, Finisher>(&self, worker: Worker, finisher: Finisher)
    where
        Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
        WorkOutput: Send + 'static,
        Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
    {
        // This function runs in a worker (non-GUI) thread. It executes the
        // work item that was provided to run_background (the `worker` value).
        //
        // The `parameter` points to BackgroundContext<...>. It owns an Arc
        // strong reference count.
        unsafe extern "system" fn thread_routine<Worker, WorkOutput, Finisher>(
            parameter: *mut c_void,
        ) -> u32
        where
            Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
            WorkOutput: Send + 'static,
            Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
        {
            let context: Box<BackgroundContext<Worker, WorkOutput, Finisher>> =
                Box::from_raw(parameter as *mut BackgroundContext<Worker, WorkOutput, Finisher>);

            // Unsafely read the worker from the context object, and then
            // execute it. Then unsafely write its output to context.output.
            let worker: Worker = context.worker.as_ptr().read();
            let output = catch_unwind(AssertUnwindSafe(worker));

            let shared_state = Arc::clone(&context.shared_state);

            if let Ok(mut guard) = shared_state.lock() {
                context.output.get().write(MaybeUninit::new(output));
                let need_wake = guard.run_on_main.is_empty();
                let hwnd = guard.hwnd;
                guard.run_on_main.push_back(context);
                drop(guard);

                // If necessary, unblock the main thread. Note that there is a
                // race condition here. The GUI thread could tear down the
                // messaging window, while this work item is running. That means
                // the window handle could become invalid. In that case, the
                // most likely thing that would happen is that the PostMessage()
                // call would use an invalid handle, but this would be safely
                // detected. Still, it would be great if we could find a way
                // to avoid this race condition.
                if need_wake {
                    PostMessageW(hwnd, MESSENGER_WM_POLL_PIPE_RECEIVERS, WPARAM(0), LPARAM(0));
                }
            } else {
                // This is a pretty bad outcome. It means that another thread
                // poisoned the mutex (panicked while holding the mutex).
                // In this case, the finisher is never going to run, because we
                // can't get a message back to the main thread.
                warn!(
                    "Failed to acquire shared state. Output of background task will be discarded."
                );
                drop(output);
            }

            0
        }

        unsafe {
            let context: Box<BackgroundContext<Worker, WorkOutput, Finisher>> =
                Box::new(BackgroundContext {
                    header: BackgroundContextHeader {
                        // completion_func: completion_func::<Worker, WorkOutput, Finisher>,
                    },
                    hwnd: self.state.hwnd,
                    worker: MaybeUninit::new(worker),
                    finisher: UnsafeCell::new(MaybeUninit::new(finisher)),
                    output: UnsafeCell::new(MaybeUninit::zeroed()),
                    shared_state: self.state.shared_state.clone(),
                });
            let context_ptr = Box::into_raw(context);

            if QueueUserWorkItem(
                Some(thread_routine::<Worker, WorkOutput, Finisher>),
                Some(context_ptr as *mut c_void),
                WORKER_THREAD_FLAGS(0),
            )
            .into()
            {
                trace!("run_background: queued work item in background thread");
            } else {
                error!("run_background: failed to queue item to background thread");
                // The work item never got queued into the background thread.
                // Pull out the live fields and dispose of them manually.
                let context = Box::from_raw(context_ptr);
                drop(context.worker.assume_init());
                // do not drop 'output'; it is not live.
                drop(context.finisher.get().read().assume_init());
            }
        }
    }
}

/// Allows a value (that does not implement `Send`) to be passed from a UI
/// thread to a different thread. The smuggled object can only be opened on
/// the same original thread that created the smuggled object.
///
/// If the `Drop` handler for this type runs, then the contained value is leaked.
/// This is required because the contained value can only safely be used on its
/// original thread.
#[cfg(todo)]
pub struct Smuggled<T> {
    cell: core::mem::MaybeUninit<T>,
}

pub struct FuncMessageReceiver<F, M>
where
    F: Fn(M) + 'static,
{
    f: F,
    fake: PhantomData<M>,
}

impl<F, M> MessageReceiver<M> for FuncMessageReceiver<F, M>
where
    M: Send + 'static,
    F: Fn(M) + 'static,
{
    fn message(&self, message: M) {
        (self.f)(message);
    }
}

impl Messenger {
    pub fn register_receiver_func<M, F>(&self, debug_description: &str, handler: F) -> Sender<M>
    where
        M: Send + 'static,
        F: Fn(M) + 'static,
    {
        self.register_receiver::<M>(
            debug_description,
            Rc::new(FuncMessageReceiver::<F, M> {
                f: handler,
                fake: PhantomData,
            }),
        )
    }

    /// Creates a worker thread and a channel from the UI thread to that worker
    /// thread. The caller provides the implementation of the worker thread.
    ///
    /// This creates a half-duplex channel. The UI thread can send messages to
    /// the worker thread, but no facility is provided for receiving responses.
    pub fn create_worker_thread_half_duplex<M, W>(
        &self,
        _debug_description: &str,
        worker: W,
    ) -> mpsc::Sender<M>
    where
        M: Send + 'static,
        W: Send + 'static + FnOnce(mpsc::Receiver<M>),
    {
        let (tx, rx) = mpsc::channel::<M>();

        let _joiner = std::thread::spawn(move || {
            worker(rx);
        });

        tx
    }

    /// Creates a worker thread, a command channel (to send commands to the
    /// worker thread) and a response channel.
    ///
    /// * `M`: type of messages sent to the worker
    /// * `W`: worker implementation
    /// * `R`: type of messages sent in response
    /// * `H`: function type which receives responses, in UI thread
    pub fn create_worker_thread_full_duplex<Command, Response, Worker, H>(
        &self,
        debug_description: &str,
        worker: Worker,
        response_handler: H,
    ) -> mpsc::Sender<Command>
    where
        Command: Send + 'static,
        Response: Send + 'static,
        Worker: Send + 'static + FnOnce(mpsc::Receiver<Command>, Sender<Response>),
        H: Fn(Response) + 'static,
    {
        let response_tx =
            self.register_receiver_func::<Response, H>(debug_description, response_handler);

        let (command_tx, command_rx) = mpsc::channel::<Command>();

        let _joiner = std::thread::spawn(move || {
            worker(command_rx, response_tx);
        });

        command_tx
    }

    pub fn register_receiver<M>(
        &self,
        debug_description: &str,
        handler: Rc<dyn MessageReceiver<M>>,
    ) -> Sender<M>
    where
        M: Send + 'static,
    {
        let queue = Arc::new(QueueState::<M> {
            messages: Mutex::new(VecDeque::new()),
            hwnd: self.state.hwnd,
        });

        // Create the UI-side state.
        let queue_receiver = Rc::new(ReceiverUIState::<M> {
            handler,
            queue: Arc::clone(&queue),
            debug_description: debug_description.to_string(),
        });

        {
            // This makes the queue discoverable by the UI thread.
            let mut receivers = self.state.receivers.borrow_mut();
            receivers.push(queue_receiver);
        }

        Sender { queue }
    }
}

trait QueueReceiver {
    fn run_on_main_thread(&self);
}

impl<M> QueueReceiver for ReceiverUIState<M> {
    // This runs on the UI thread, when we receive MESSENGER_WM_POLL_RECEIVERS.
    fn run_on_main_thread(&self) {
        trace!("polling receiver: {}", self.debug_description);
        loop {
            let mut messages = self.queue.messages.lock().unwrap();
            if let Some(message) = messages.pop_front() {
                drop(messages); // unlock -- very important!
                self.handler.message(message);
            } else {
                // receiver is done
                break;
            }
        }
    }
}

// this is kept on the UI side
struct ReceiverUIState<M> {
    queue: Arc<QueueState<M>>,
    handler: Rc<dyn MessageReceiver<M>>,
    debug_description: String,
}

struct QueueState<M> {
    messages: Mutex<VecDeque<M>>,
    hwnd: HWND,
}

pub trait MessageReceiver<M> {
    fn message(&self, message: M);
    fn closed(&self) {}
}

/// An object which can send messages.
pub struct Sender<M> {
    queue: Arc<QueueState<M>>,
}

impl<M> Sender<M> {
    /// Sends a message to the UI thread. This will wake up the UI thread,
    /// if necessary.
    pub fn send(&self, message: M) {
        let mut messages = self.queue.messages.lock().unwrap();
        let was_empty = messages.is_empty();
        messages.push_back(message);
        drop(messages);

        if was_empty {
            unsafe {
                PostMessageW(
                    self.queue.hwnd,
                    MESSENGER_WM_POLL_PIPE_RECEIVERS,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }
}
