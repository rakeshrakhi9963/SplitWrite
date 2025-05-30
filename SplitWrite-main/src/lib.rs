
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    broken_intra_doc_links
)]
#![allow(clippy::type_complexity)]

mod sync;

use crate::sync::{Arc, AtomicUsize, Mutex};

type Epochs = Arc<Mutex<slab::Slab<Arc<AtomicUsize>>>>;

mod write;
pub use crate::write::Taken;
pub use crate::write::WriteHandle;

mod read;
pub use crate::read::{ReadGuard, ReadHandle, ReadHandleFactory};

pub mod aliasing;


pub trait Absorb<O> {
   
    fn absorb_first(&mut self, operation: &mut O, other: &Self);

    fn absorb_second(&mut self, mut operation: O, other: &Self) {
        Self::absorb_first(self, &mut operation, other)
    }

    #[allow(clippy::boxed_local)]
    fn drop_first(self: Box<Self>) {}
    #[allow(clippy::boxed_local)]
    fn drop_second(self: Box<Self>) {}

    fn sync_with(&mut self, first: &Self);
}
pub fn new_from_empty<T, O>(t: T) -> (WriteHandle<T, O>, ReadHandle<T>)
where
    T: Absorb<O> + Clone,
{
    let epochs = Default::default();

    let r = ReadHandle::new(t.clone(), Arc::clone(&epochs));
    let w = WriteHandle::new(t, epochs, r.clone());
    (w, r)
}

pub fn new<T, O>() -> (WriteHandle<T, O>, ReadHandle<T>)
where
    T: Absorb<O> + Default,
{
    let epochs = Default::default();

    let r = ReadHandle::new(T::default(), Arc::clone(&epochs));
    let w = WriteHandle::new(T::default(), epochs, r.clone());
    (w, r)
}
