use std::{
    any::Any,
    ffi::c_void,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use pin_project::pin_project;
use windows::{
    Win32::System::Threading::{PTP_CALLBACK_INSTANCE, TrySubmitThreadpoolCallback},
    core::Error as WindowsError,
};

pub fn submit_work<F, Output>(work: F) -> impl Future<Output = Result<F::Output, ThreadPoolError>>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    Work::new(work)
}

#[derive(Debug)]
pub enum ThreadPoolError {
    Runtime(WindowsError),
    Panic(Box<dyn Any + Send>),
}

impl ThreadPoolError {
    pub fn err_or_resume_unwind(self) -> WindowsError {
        match self {
            ThreadPoolError::Runtime(error) => error,
            ThreadPoolError::Panic(payload) => std::panic::resume_unwind(payload),
        }
    }
}

#[pin_project(project = WorkProj)]
enum Work<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    Initial(Option<F>),
    Running(Arc<Mutex<WorkState<F, Output>>>),
    Done,
}

#[derive(Debug)]
struct WorkState<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    f: Option<F>,
    waker: Option<Waker>,
    result: Option<Result<F::Output, ThreadPoolError>>,
}

impl<F, Output> Work<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    pub fn new(f: F) -> Self {
        Self::Initial(Some(f))
    }

    unsafe extern "system" fn callback(_instance: PTP_CALLBACK_INSTANCE, context: *mut c_void) {
        let state = unsafe { Arc::from_raw(context as *const Mutex<WorkState<F, Output>>) };
        let f = {
            let mut state = match state.lock() {
                Ok(locked) => locked,
                Err(poisoned) => poisoned.into_inner(),
            };
            state.f.take()
        };
        let f = f.expect("bug: missing function");
        let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || f()));
        {
            let mut state = match state.lock() {
                Ok(locked) => locked,
                Err(poisoned) => poisoned.into_inner(),
            };
            state.result = Some(output.map_err(|err| ThreadPoolError::Panic(err)));
            state.waker.take().map(|waker| waker.wake());
        };
    }
}

impl<F, Output> Future for Work<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    type Output = Result<F::Output, ThreadPoolError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (new_state, poll) = match self.as_mut().project() {
            WorkProj::Initial(f) => {
                let state = WorkState { f: f.take(), waker: Some(cx.waker().clone()), result: None };
                let state = Arc::new(Mutex::new(state));
                let state_ptr = Arc::into_raw(state.clone());
                match unsafe { TrySubmitThreadpoolCallback(Some(Self::callback), Some(state_ptr as *mut c_void), None) }
                {
                    Ok(_) => (Some(Self::Running(state)), Poll::Pending),
                    Err(err) => {
                        let _ = unsafe { Arc::from_raw(state_ptr) };
                        (Some(Self::Done), Poll::Ready(Err(ThreadPoolError::Runtime(err))))
                    }
                }
            }
            WorkProj::Running(state) => {
                let mut state = state.lock().unwrap();
                match state.result.take() {
                    Some(result) => (Some(Self::Done), Poll::Ready(result)),
                    None => {
                        state.waker = Some(cx.waker().clone());
                        (None, Poll::Pending)
                    }
                }
            }
            WorkProj::Done => panic!("future already completed"),
        };
        if let Some(new_state) = new_state {
            self.set(new_state);
        }
        poll
    }
}
