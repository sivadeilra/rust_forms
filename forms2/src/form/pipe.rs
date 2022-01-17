use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::{atomic::Ordering::SeqCst, Arc, Mutex};

use super::*;

impl Form {
    pub fn register_receiver_func<M, F>(&self, debug_description: &str, handler: F) -> Sender<M>
    where
        M: Send + 'static,
        F: Fn(M) + 'static,
    {
        use core::marker::PhantomData;

        struct MessageHandler<F, M>
        where
            F: Fn(M) + 'static,
        {
            f: F,
            fake: PhantomData<M>,
        }

        impl<F, M> MessageReceiver<M> for MessageHandler<F, M>
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
            Rc::new(MessageHandler::<F, M> {
                f: handler,
                fake: PhantomData,
            }),
        )
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
            closed: AtomicBool::new(false),
            receiver_running: AtomicBool::new(false),
            // receiver: receiver_box_raw,
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
            let mut receivers = self.receivers.borrow();
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
        debug!("polling receiver: {}", self.debug_description);

        // self.queue.receiver_running.store(true, SeqCst);
        loop {
            let mut messages = self.queue.messages.lock().unwrap();
            if let Some(message) = messages.pop_front() {
                drop(messages); // unlock -- very important!
                trace!("received one message");
                self.handler.message(message);
            } else {
                trace!("receiver done");
                break;
            }
        }
        // self.queue.receiver_running.store(false, SeqCst);
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
    closed: AtomicBool,
    receiver_running: AtomicBool,
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
