//! Allows a GUI thread to receive messages from other threads.
//!

use super::*;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Messenger {
    state: Rc<MessengerState>,
}

static_assertions::assert_not_impl_any!(Messenger: Send, Sync);

struct MessengerState {
    hwnd: HWND,
    receivers: RefCell<Vec<Rc<dyn QueueReceiver>>>,
    receiver: mpsc::Receiver<Message>,
    sender: mpsc::Sender<Message>,
}

enum Message {
    RunThis(Arc<dyn Fn()>),
}

struct AtomicState {
    receivers: VecDeque<Arc<dyn QueueReceiver>>,
}

impl Messenger {
    pub fn new() -> Messenger {
        let (tx, rx) = mpsc::channel::<Message>();

        unsafe {
            // Create the messaging window.
            let window_class_atom = register_class_lazy();
            let instance = get_instance();
            let window_name: [u16; 2] = [0; 2];
            let hwnd = CreateWindowExW(
                0,
                PWSTR(window_class_atom as usize as *mut u16),
                PWSTR(window_name.as_ptr() as *mut _), // window name
                0,                                     // style
                0,                                     // x
                0,                                     // y
                1,                                     // width
                1,                                     // height
                Some(HWND_MESSAGE as isize),
                None,
                instance,
                null(),
            );
            if hwnd == 0 {
                panic!("Failed to create messaging window for AsyncExecutor");
            }

            let state: Rc<MessengerState> = Rc::new(MessengerState {
                hwnd,
                receivers: Default::default(),
                receiver: rx,
                sender: tx,
            });
            let state_ptr: *const MessengerState = &*state;

            SetWindowLongPtrW(hwnd, 0, state_ptr as LPARAM);

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

        let mut class_name_wstr = U16CString::from_str(EXECUTOR_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PWSTR(class_name_wstr.as_mut_ptr());
        class_ex.style = 0;
        class_ex.hbrBackground = 0;
        class_ex.lpfnWndProc = Some(messenger_wndproc);
        class_ex.hCursor = 0;
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
                return 1;
            }

            _ => {}
        }

        let state_ptr: isize = GetWindowLongPtrW(hwnd, 0);
        if state_ptr == 0 {
            return DefWindowProcW(hwnd, message, wparam, lparam);
        }

        let state: *const MessengerState = state_ptr as *const MessengerState;

        match message {
            MESSENGER_WM_BACKGROUND_COMPLETION => {
                trace!("MESSENGER_WM_BACKGROUND_COMPLETION");
                let header = lparam as *mut BackgroundContextHeader;
                let completion_func = (*header).completion_func;
                completion_func(header);
                return 0;
            }

            MESSENGER_WM_POLL_PIPE_RECEIVERS => {
                trace!("MESSENGER_WM_POLL_PIPE_RECEIVERS");
                (*state).poll_receivers();

                let state = &*state;
                while let Ok(message) = state.receiver.try_recv() {
                    match message {
                        Message::RunThis(run_this) => {
                            run_this();
                            drop(run_this);
                        }
                    }
                }
                return 0;
            }

            WM_DESTROY => {}

            _ => {
                // allow default to run
            }
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
    completion_func: unsafe fn(header: *mut BackgroundContextHeader),
}

#[repr(C)] // want linear layout
struct BackgroundContext<Worker, WorkOutput, Finisher> {
    header: BackgroundContextHeader,
    hwnd: HWND,
    worker: MaybeUninit<Worker>,
    output: MaybeUninit<std::thread::Result<WorkOutput>>,
    finisher: MaybeUninit<Finisher>,
}

impl Messenger {
    /// Run some work on a background thread. When the work completes, run
    /// a function which completes the work, on the UI thread.
    ///
    /// This is a "one-shot" background request.
    pub fn run_background<Worker, WorkOutput, Finisher>(&self, worker: Worker, finisher: Finisher)
    where
        Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
        Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
    {
        unsafe extern "system" fn thread_routine<Worker, WorkOutput, Finisher>(
            parameter: *mut c_void,
        ) -> u32
        where
            Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
            Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
        {
            let mut context = parameter as *mut BackgroundContext<Worker, WorkOutput, Finisher>;

            (*context).output = MaybeUninit::new(catch_unwind(AssertUnwindSafe(move || {
                let worker: Worker = (*context).worker.as_mut_ptr().read();
                worker()
            })));

            // Now post it back to the main thread.
            PostMessageW(
                (*context).hwnd,
                MESSENGER_WM_BACKGROUND_COMPLETION,
                0,
                parameter as LPARAM,
            );

            0
        }

        unsafe fn completion_func<Worker, WorkOutput, Finisher>(
            parameter: *mut BackgroundContextHeader,
        ) where
            Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
            Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
        {
            // In this state:
            // - 'worker' field is dead
            // - 'output' field is live
            // - 'finisher' field is live
            let context = parameter as *mut BackgroundContext<Worker, WorkOutput, Finisher>;

            let finisher = (*context).finisher.as_mut_ptr().read();
            let output = (*context).output.as_mut_ptr().read();

            // Free the memory for the context allocation before calling the
            // finisher function. That way, we don't need to worry about what
            // happens if that function panics. Since we know the full type,
            // we can safely drop the box.
            drop(Box::from_raw(context));

            finisher(output);
        }

        unsafe {
            let context: Box<BackgroundContext<Worker, WorkOutput, Finisher>> =
                Box::new(BackgroundContext {
                    header: BackgroundContextHeader {
                        completion_func: completion_func::<Worker, WorkOutput, Finisher>,
                    },
                    hwnd: self.state.hwnd,
                    worker: MaybeUninit::new(worker),
                    finisher: MaybeUninit::new(finisher),
                    output: MaybeUninit::zeroed(),
                });
            let context_ptr = Box::into_raw(context);

            if QueueUserWorkItem(
                Some(thread_routine::<Worker, WorkOutput, Finisher>),
                context_ptr as *mut c_void,
                0,
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
                drop(context.finisher.assume_init());
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
        let queue_state = Arc::new(QueueState::<M> {
            messages: Mutex::new(VecDeque::new()),
            hwnd: self.state.hwnd,
        });

        // Create the UI-side state.
        let queue_receiver = Rc::new(ReceiverUIState::<M> {
            handler: handler,
            queue: Arc::clone(&queue_state),
            debug_description: debug_description.to_string(),
        });

        {
            // This makes the queue discoverable by the UI thread.
            let mut receivers = self.state.receivers.borrow_mut();
            receivers.push(queue_receiver);
        }

        Sender { queue: queue_state }
    }
}

impl MessengerState {
    // This runs in response to FORM_WM_POLL_PIPE_RECEIVERS. It polls all of them.
    pub(crate) fn poll_receivers(&self) {
        let mut i = 0;
        loop {
            let receivers = self.receivers.borrow();
            if i >= receivers.len() {
                break;
            }

            let receiver_rc = Rc::clone(&receivers[i]);
            drop(receivers); // drop borrow
            i += 1;

            // We drop the dynamic borrow of the receivers collection so that
            // we can safely call into this app callback, without worrying about
            // the app modifying the receiver set.
            receiver_rc.run_on_main_thread();
        }
    }
}

trait QueueReceiver {
    fn run_on_main_thread(&self);
}

impl<M> QueueReceiver for ReceiverUIState<M> {
    // This runs on the UI thread, when we receive FORM_WM_POLL_RECEIVERS.
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
                PostMessageW(self.queue.hwnd, MESSENGER_WM_POLL_PIPE_RECEIVERS, 0, 0);
            }
        }
    }
}
