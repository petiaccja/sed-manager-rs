use std::{
    any::Any,
    cell::UnsafeCell,
    ffi::c_void,
    ops::DerefMut,
    pin::Pin,
    sync::{Arc, Mutex, atomic::AtomicPtr},
    task::{Context, Poll, Waker},
};

use pin_project::pin_project;
use windows::{
    Win32::{
        Foundation::{ERROR_IO_PENDING, HANDLE},
        Storage::FileSystem::SetFileCompletionNotificationModes,
        System::{
            IO::OVERLAPPED,
            Threading::{
                CancelThreadpoolIo, CloseThreadpoolIo, CreateThreadpoolIo, PTP_CALLBACK_INSTANCE, PTP_IO,
                StartThreadpoolIo, TrySubmitThreadpoolCallback, WaitForThreadpoolIoCallbacks,
            },
            WindowsProgramming::{FILE_SKIP_COMPLETION_PORT_ON_SUCCESS, FILE_SKIP_SET_EVENT_ON_HANDLE},
        },
    },
    core::{Error as WindowsError, HRESULT},
};

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

pub fn submit_work<F, Output>(work: F) -> impl Future<Output = Result<F::Output, ThreadPoolError>>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    Work::new(work)
}

pub struct ThreadPoolIo {
    tp_io: PTP_IO,
}

impl ThreadPoolIo {
    pub fn new(device: HANDLE) -> Result<Self, WindowsError> {
        unsafe {
            SetFileCompletionNotificationModes(
                device,
                (FILE_SKIP_COMPLETION_PORT_ON_SUCCESS | FILE_SKIP_SET_EVENT_ON_HANDLE) as u8,
            )
        }?;
        let tp_io = unsafe { CreateThreadpoolIo(device, Some(overlapped_io_callback), None, None) }?;
        Ok(Self { tp_io })
    }

    pub fn submit<F>(&self, io: F) -> impl Future<Output = Result<u32, WindowsError>>
    where
        F: FnOnce(AtomicPtr<OVERLAPPED>) -> Result<u32, WindowsError>,
    {
        OverlappedIo::new(self.tp_io, io)
    }
}

impl Drop for ThreadPoolIo {
    fn drop(&mut self) {
        unsafe {
            WaitForThreadpoolIoCallbacks(self.tp_io, false);
            CloseThreadpoolIo(self.tp_io);
        };
    }
}

//------------------------------------------------------------------------------
// Utilities.
//------------------------------------------------------------------------------

#[derive(Debug)]
enum FutureState<Output> {
    Waiting(Waker),
    Done(Output),
}

//------------------------------------------------------------------------------
// Future for thread-pool work.
//------------------------------------------------------------------------------

#[pin_project(project = WorkProj)]
enum Work<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    Initial(Option<F>),
    Running(Arc<Mutex<WorkContext<F, Output>>>),
    Done,
}

impl<F, Output> Work<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    pub fn new(f: F) -> Self {
        Self::Initial(Some(f))
    }

    extern "system" fn callback(_instance: PTP_CALLBACK_INSTANCE, context: *mut c_void) {
        let context = unsafe { Arc::from_raw(context as *const Mutex<WorkContext<F, Output>>) };
        let f = {
            let mut state = match context.lock() {
                Ok(locked) => locked,
                Err(poisoned) => poisoned.into_inner(),
            };
            state.f.take()
        };
        let f = f.expect("bug: missing function");
        let output = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || f()));
        {
            let mut context = match context.lock() {
                Ok(locked) => locked,
                Err(poisoned) => poisoned.into_inner(),
            };
            let state = std::mem::replace(
                &mut context.state,
                FutureState::Done(output.map_err(|err| ThreadPoolError::Panic(err))),
            );
            if let FutureState::Waiting(waker) = state {
                waker.wake();
            }
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
                let state = WorkContext { f: f.take(), state: FutureState::Waiting(cx.waker().clone()) };
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
            WorkProj::Running(context) => {
                let mut context = context.lock().unwrap();
                match core::mem::replace(&mut context.state, FutureState::Waiting(cx.waker().clone())) {
                    FutureState::Done(result) => (Some(Self::Done), Poll::Ready(result)),
                    _ => (None, Poll::Pending),
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

#[derive(Debug)]
struct WorkContext<F, Output>
where
    F: FnOnce() -> Output + Send,
    F::Output: Send,
{
    f: Option<F>,
    state: FutureState<Result<F::Output, ThreadPoolError>>,
}

//------------------------------------------------------------------------------
// Future for overlapped I/O.
//------------------------------------------------------------------------------

#[pin_project(project = OverlappedIoProj, project_replace = OverlappedIoCopy)]
enum OverlappedIo<F>
where
    F: FnOnce(AtomicPtr<OVERLAPPED>) -> Result<u32, WindowsError>,
{
    Initial(PTP_IO, F),
    Running(Arc<OverlappedIoContext>),
    Done,
}

impl<F> OverlappedIo<F>
where
    F: FnOnce(AtomicPtr<OVERLAPPED>) -> Result<u32, WindowsError>,
{
    pub fn new(tp_io: PTP_IO, io: F) -> Self {
        Self::Initial(tp_io, io)
    }
}

impl<F> Future for OverlappedIo<F>
where
    F: FnOnce(AtomicPtr<OVERLAPPED>) -> Result<u32, WindowsError>,
{
    type Output = Result<u32, WindowsError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (state, poll) = match self.as_mut().project_replace(OverlappedIo::Done) {
            OverlappedIoCopy::Initial(tp_io, io) => {
                let context = Arc::new(OverlappedIoContext {
                    overlapped: OVERLAPPED::default().into(),
                    state: Mutex::new(FutureState::Waiting(cx.waker().clone())),
                });
                let raw_context = Arc::into_raw(context.clone());

                unsafe { StartThreadpoolIo(tp_io) };
                match io(context.overlapped.get().into()) {
                    Ok(value) => {
                        unsafe { CancelThreadpoolIo(tp_io) };
                        unsafe { drop(Arc::from_raw(raw_context)) };
                        (OverlappedIo::Done, Poll::Ready(Ok(value)))
                    }
                    Err(err) if err.code() == HRESULT::from_win32(ERROR_IO_PENDING.0) => {
                        (OverlappedIo::Running(context), Poll::Pending)
                    }
                    Err(err) => {
                        unsafe { CancelThreadpoolIo(tp_io) };
                        unsafe { drop(Arc::from_raw(raw_context)) };
                        (OverlappedIo::Done, Poll::Ready(Err(err)))
                    }
                }
            }
            OverlappedIoCopy::Running(context) => {
                let state = {
                    let mut guard = context.state.lock().unwrap();
                    core::mem::replace(guard.deref_mut(), FutureState::Waiting(cx.waker().clone()))
                };
                match state {
                    FutureState::Done(result) => (Self::Done, Poll::Ready(result)),
                    _ => (OverlappedIo::Running(context), Poll::Pending),
                }
            }
            OverlappedIoCopy::Done => panic!("future already completed"),
        };

        let _ = self.as_mut().project_replace(state);
        poll
    }
}

#[repr(C)]
struct OverlappedIoContext {
    overlapped: UnsafeCell<OVERLAPPED>,
    state: Mutex<FutureState<Result<u32, WindowsError>>>,
}

unsafe impl Send for OverlappedIoContext {}
unsafe impl Sync for OverlappedIoContext {}

extern "system" fn overlapped_io_callback(
    _instance: PTP_CALLBACK_INSTANCE,
    _context: *mut c_void,
    overlapped: *mut c_void,
    win32_result: u32,
    num_bytes_transferred: usize,
    _io: PTP_IO,
) {
    let context = unsafe { Arc::from_raw(overlapped as *const OverlappedIoContext) };
    let result = match win32_result {
        0 => Ok(num_bytes_transferred as u32),
        error => Err(WindowsError::from_hresult(HRESULT::from_win32(error))),
    };

    let mut state = match context.state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    match core::mem::replace(state.deref_mut(), FutureState::Done(result)) {
        FutureState::Waiting(waker) => waker.wake(),
        FutureState::Done(_) => (),
    };
}

#[cfg(test)]
mod tests {
    use crate::windows::device_handle::DeviceHandle;

    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};

    use googletest::assert_that;
    use googletest::matchers::*;
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::Storage::FileSystem::*;
    use windows::Win32::System::Pipes::*;
    use windows::core::HSTRING;

    static PIPE_ID: AtomicU64 = AtomicU64::new(0);

    #[tokio::test]
    async fn submit_work_success() {
        assert_eq!(submit_work(|| 3 + 4).await.unwrap(), 7);
    }

    #[tokio::test]
    async fn submit_work_panic() {
        assert_that!(submit_work(|| panic!()).await, err(matches_pattern!(ThreadPoolError::Panic(_))));
    }

    fn create_pipes() -> (DeviceHandle, DeviceHandle) {
        let name = HSTRING::from(format!(r"\\.\pipe\test-async-io-{}", PIPE_ID.fetch_add(1, Ordering::Relaxed)));

        let read_handle = unsafe {
            CreateNamedPipeW(
                &name,
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                None,
            )
        };

        let write_handle = unsafe {
            CreateFileW(
                &name,
                (GENERIC_WRITE | GENERIC_READ).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                None,
            )
            .unwrap()
        };

        (DeviceHandle(read_handle), DeviceHandle(write_handle))
    }

    #[tokio::test]
    async fn overlapped_io_success_immediate() {
        let (read_handle, write_handle) = create_pipes();
        let read_io = ThreadPoolIo::new(read_handle.0).unwrap();
        let write_io = ThreadPoolIo::new(write_handle.0).unwrap();

        let write_buffer = [1, 2, 3];
        let write_result = write_io
            .submit(|overlapped| {
                let mut num_bytes_transferred = 0;
                unsafe {
                    WriteFile(
                        write_handle.0,
                        Some(write_buffer.as_slice()),
                        Some(&mut num_bytes_transferred as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| num_bytes_transferred)
            })
            .await;

        assert_eq!(write_result, Ok(3));

        let mut read_buffer = vec![0, 0, 0];
        let read_result = read_io
            .submit(|overlapped| {
                let mut num_bytes_transferred = 0;
                unsafe {
                    ReadFile(
                        read_handle.0,
                        Some(read_buffer.as_mut_slice()),
                        Some(&mut num_bytes_transferred as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| num_bytes_transferred)
            })
            .await;

        assert_eq!(read_result, Ok(3));
        assert_eq!(read_buffer, write_buffer);
    }

    #[tokio::test]
    async fn overlapped_io_success_drop_future() {
        let (read_handle, write_handle) = create_pipes();
        let read_io = ThreadPoolIo::new(read_handle.0).unwrap();
        let write_io = ThreadPoolIo::new(write_handle.0).unwrap();

        let write_buffer = [1, 2, 3];
        let mut context = Context::from_waker(Waker::noop());
        let _ = core::pin::pin!(write_io.submit(|overlapped| {
            let mut num_bytes_transferred = 0;
            unsafe {
                WriteFile(
                    write_handle.0,
                    Some(write_buffer.as_slice()),
                    Some(&mut num_bytes_transferred as *mut _),
                    Some(overlapped.load(Ordering::Relaxed)),
                )
            }
            .map(|_| num_bytes_transferred)
        }))
        .poll(&mut context);

        let mut read_buffer = vec![0, 0, 0];
        let read_result = read_io
            .submit(|overlapped| {
                let mut num_bytes_transferred = 0;
                unsafe {
                    ReadFile(
                        read_handle.0,
                        Some(read_buffer.as_mut_slice()),
                        Some(&mut num_bytes_transferred as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| num_bytes_transferred)
            })
            .await;

        assert_eq!(read_result, Ok(3));
        assert_eq!(read_buffer, write_buffer);
    }

    #[tokio::test]
    async fn overlapped_io_success_delayed() {
        let (read_handle, write_handle) = create_pipes();
        let read_io = ThreadPoolIo::new(read_handle.0).unwrap();
        let write_io = ThreadPoolIo::new(write_handle.0).unwrap();

        let mut context = Context::from_waker(Waker::noop());

        let mut read_buffer = vec![0, 0, 0];
        let write_buffer = [1, 2, 3];

        let read_result = {
            let read_future = read_io.submit(|overlapped| {
                let mut num_bytes_transferred = 0;
                unsafe {
                    ReadFile(
                        read_handle.0,
                        Some(read_buffer.as_mut_slice()),
                        Some(&mut num_bytes_transferred as *mut _),
                        Some(overlapped.load(Ordering::Relaxed)),
                    )
                }
                .map(|_| num_bytes_transferred)
            });

            let mut read_future = std::pin::pin!(read_future);
            assert_that!(read_future.as_mut().poll(&mut context), matches_pattern!(Poll::Pending));

            let write_result = write_io
                .submit(|overlapped| {
                    let mut num_bytes_transferred = 0;
                    unsafe {
                        WriteFile(
                            write_handle.0,
                            Some(write_buffer.as_slice()),
                            Some(&mut num_bytes_transferred as *mut _),
                            Some(overlapped.load(Ordering::Relaxed)),
                        )
                    }
                    .map(|_| num_bytes_transferred)
                })
                .await;

            assert_eq!(write_result, Ok(3));

            read_future.await
        };
        assert_eq!(read_result, Ok(3));
        assert_eq!(read_buffer, write_buffer);
    }
}
