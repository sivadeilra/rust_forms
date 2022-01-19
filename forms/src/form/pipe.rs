use super::*;
use core::marker::PhantomData;
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

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

impl Form {
    pub fn register_receiver_func<M, F>(&self, debug_description: &str, handler: F) -> Sender<M>
    where
        M: Send + 'static,
        F: Fn(M) + 'static,
    {
        struct FuncMessageReceiver<F, M>
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
    pub fn create_worker_thread_full_duplex<Command, Response, Worker>(
        &self,
        debug_description: &str,
        worker: Worker,
        response_handler: Rc<dyn MessageReceiver<Response>>,
    ) -> mpsc::Sender<Command>
    where
        Command: Send + 'static,
        Response: Send + 'static,
        Worker: Send + 'static + FnOnce(mpsc::Receiver<Command>, Sender<Response>),
    {
        let response_tx = self.register_receiver::<Response>(debug_description, response_handler);

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
            hwnd: self.state.handle.get(),
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

impl FormState {
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
            receiver_rc.receive();
        }
    }
}

pub(crate) trait QueueReceiver {
    fn receive(&self);
}

impl<M> QueueReceiver for ReceiverUIState<M> {
    // This runs on the UI thread, when we receive FORM_WM_POLL_RECEIVERS.
    fn receive(&self) {
        trace!("polling receiver: {}", self.debug_description);
        loop {
            let mut messages = self.queue.messages.lock().unwrap();
            if let Some(message) = messages.pop_front() {
                drop(messages); // unlock -- very important!
                                // trace!("received one message");
                self.handler.message(message);
            } else {
                // trace!("receiver done");
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
    // receiver: *mut dyn MessageReceiver<M>,
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
                PostMessageW(self.queue.hwnd, FORM_WM_POLL_PIPE_RECEIVERS, 0, 0);
            }
        }
    }
}
