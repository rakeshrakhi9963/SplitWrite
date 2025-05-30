use crate::sync::{fence, Arc, AtomicPtr, AtomicUsize, Ordering};
use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

#[cfg(doc)]
use crate::WriteHandle;

mod guard;
pub use guard::ReadGuard;

mod factory;
pub use factory::ReadHandleFactory;

pub struct ReadHandle<T> {
    pub(crate) inner: Arc<AtomicPtr<T>>,
    pub(crate) epochs: crate::Epochs,
    epoch: Arc<AtomicUsize>,
    epoch_i: usize,
    enters: Cell<usize>,

    _unimpl_send: PhantomData<*const T>,
}
unsafe impl<T> Send for ReadHandle<T> where T: Sync {}

impl<T> Drop for ReadHandle<T> {
    fn drop(&mut self) {
        let e = self.epochs.lock().unwrap().remove(self.epoch_i);
        assert!(Arc::ptr_eq(&e, &self.epoch));
        assert_eq!(self.enters.get(), 0);
    }
}

impl<T> fmt::Debug for ReadHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadHandle")
            .field("epochs", &self.epochs)
            .field("epoch", &self.epoch)
            .finish()
    }
}

impl<T> Clone for ReadHandle<T> {
    fn clone(&self) -> Self {
        ReadHandle::new_with_arc(Arc::clone(&self.inner), Arc::clone(&self.epochs))
    }
}

impl<T> ReadHandle<T> {
    pub(crate) fn new(inner: T, epochs: crate::Epochs) -> Self {
        let store = Box::into_raw(Box::new(inner));
        let inner = Arc::new(AtomicPtr::new(store));
        Self::new_with_arc(inner, epochs)
    }

    fn new_with_arc(inner: Arc<AtomicPtr<T>>, epochs: crate::Epochs) -> Self {
      
        let epoch = Arc::new(AtomicUsize::new(0));
      
        let epoch_i = epochs.lock().unwrap().insert(Arc::clone(&epoch));

        Self {
            epochs,
            epoch,
            epoch_i,
            enters: Cell::new(0),
            inner,
            _unimpl_send: PhantomData,
        }
    }

    pub fn factory(&self) -> ReadHandleFactory<T> {
        ReadHandleFactory {
            inner: Arc::clone(&self.inner),
            epochs: Arc::clone(&self.epochs),
        }
    }
}

impl<T> ReadHandle<T> {
    pub fn enter(&self) -> Option<ReadGuard<'_, T>> {
        let enters = self.enters.get();
        if enters != 0 {
            let r_handle = self.inner.load(Ordering::Acquire);
            let r_handle = unsafe { r_handle.as_ref() };

            return if let Some(r_handle) = r_handle {
                self.enters.set(enters + 1);
                Some(ReadGuard {
                    handle: guard::ReadHandleState::from(self),
                    t: r_handle,
                })
            } else {
                unreachable!("if pointer is null, no ReadGuard should have been issued");
            };
        }

        self.epoch.fetch_add(1, Ordering::AcqRel);
        fence(Ordering::SeqCst);

        let r_handle = self.inner.load(Ordering::Acquire);
        let r_handle = unsafe { r_handle.as_ref() };

        if let Some(r_handle) = r_handle {
            let enters = self.enters.get() + 1;
            self.enters.set(enters);
            Some(ReadGuard {
                handle: guard::ReadHandleState::from(self),
                t: r_handle,
            })
        } else {
            self.epoch.fetch_add(1, Ordering::AcqRel);
            None
        }
    }

    pub fn was_dropped(&self) -> bool {
        self.inner.load(Ordering::Acquire).is_null()
    }

    pub fn raw_handle(&self) -> Option<NonNull<T>> {
        NonNull::new(self.inner.load(Ordering::Acquire))
    }
}


#[allow(dead_code)]
struct CheckReadHandleSendNotSync;
