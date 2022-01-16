use super::*;
use core::mem::MaybeUninit;
use std::panic::{catch_unwind, AssertUnwindSafe};

struct BackgroundContextHeader {
    completion_func: unsafe fn(header: *mut BackgroundContextHeader),
}

struct BackgroundContext<Worker, WorkOutput, Finisher> {
    header: BackgroundContextHeader,
    hwnd: HWND,
    worker: MaybeUninit<Worker>,
    output: MaybeUninit<std::thread::Result<WorkOutput>>,
    finisher: MaybeUninit<Finisher>,
}

impl Form {
    // Run some work on a background thread. When the work completes, run
    // a function which completes the work, on the UI thread.
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
                let mut worker: Worker = (*context).worker.as_mut_ptr().read();
                worker()
            })));

            // Now post it back to the main thread.
            PostMessageW(
                (*context).hwnd,
                FORM_WM_BACKGROUND_COMPLETION,
                0,
                parameter as LPARAM,
            );

            0
        }

        unsafe fn completion_func<Worker, WorkOutput, Finisher>(
            parameter: *mut BackgroundContextHeader,
        )
        where
            Worker: FnOnce() -> WorkOutput + 'static + Sync + Sync,
            Finisher: FnOnce(std::thread::Result<WorkOutput>) + 'static,
        {
            // In this state:
            // - 'worker' field is dead
            // - 'output' field is live
            // - 'finisher' field is live
            let mut context = parameter as *mut BackgroundContext<Worker, WorkOutput, Finisher>;

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
            let mut context: Box<BackgroundContext<Worker, WorkOutput, Finisher>> =
                Box::new(BackgroundContext {
                    header: BackgroundContextHeader {
                        completion_func: completion_func::<Worker, WorkOutput, Finisher>,
                    },
                    hwnd: self.state.handle.get(),
                    worker: MaybeUninit::new(worker),
                    finisher: MaybeUninit::new(finisher),
                    output: MaybeUninit::zeroed(),
                });
            let mut context_ptr = Box::into_raw(context);

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

    pub(super) fn finish_background(lparam: LPARAM) {
        unsafe {
            let header = lparam as *mut BackgroundContextHeader;
            let completion_func = (*header).completion_func;
            completion_func(header);
        }
    }
}
