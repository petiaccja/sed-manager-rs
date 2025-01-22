/// This trait is a workaround until there is a solution for `AsyncDrop`.
///
/// Some objects in this crate need a non-trivial implementation for [`Drop`],
/// but that implementation must also call async functions. The most prominent
/// example for this is the SP session, which needs to be closed by sending
/// an EndOfSession message to the storage device, and that message needs to
/// go through the entire asynchronous protocol.
///
/// This trait formalizes the procedure for dropping such objects.
/// The process happens in two phases:
///   1. Call [`AsyncFinalize::finalize`]: this does final async tasks.
///     - This **SHOULD** manually `finalize` all sub-objects!
///   2. Call [`Drop::drop`]: Rust's regular drop mechanism.
///     - This should block on `finalize` if it hasn't been called yet!
///
/// This is far from ideal, so hopefully `AsyncDrop` will be ready soon.
#[allow(drop_bounds)]
pub trait AsyncFinalize
where
    // `Drop` should always be implemented to avoid leaking resources.
    Self: Drop,
{
    async fn finalize(&mut self);
}

/// Finalize an object asynchronously.
///
/// Unlike [`Drop::drop`], this does NOT actually drop the object.
/// The purpose of this function is to avoid blocking within [`Drop::drop`]
/// by making sure the async finalization is done beforehand.
pub async fn async_finalize<T: AsyncFinalize>(x: &mut T) {
    <T as AsyncFinalize>::finalize(x).await;
}

/// Finalize object synchronously.
///
/// Does the same as [`async_finalize`], but is meant to be called from
/// a blocking context, like [`Drop::drop`].
pub fn sync_finalize<T: AsyncFinalize>(x: &mut T) {
    tokio::task::block_in_place(|| {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(<T as AsyncFinalize>::finalize(x))
        } else {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
            runtime.block_on(<T as AsyncFinalize>::finalize(x));
        }
    })
}
