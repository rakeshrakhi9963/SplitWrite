use super::ReadHandle;
use crate::sync::{Arc, AtomicPtr};
use std::fmt;

pub struct ReadHandleFactory<T> {
    pub(super) inner: Arc<AtomicPtr<T>>,
    pub(super) epochs: crate::Epochs,
}

impl<T> fmt::Debug for ReadHandleFactory<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadHandleFactory")
            .field("epochs", &self.epochs)
            .finish()
    }
}

impl<T> Clone for ReadHandleFactory<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            epochs: Arc::clone(&self.epochs),
        }
    }
}

impl<T> ReadHandleFactory<T> {
    
    pub fn handle(&self) -> ReadHandle<T> {
        ReadHandle::new_with_arc(Arc::clone(&self.inner), Arc::clone(&self.epochs))
    }
}
