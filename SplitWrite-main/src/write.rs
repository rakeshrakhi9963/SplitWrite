use crate::read::ReadHandle;
use crate::Absorb;

use crate::sync::{fence, Arc, AtomicUsize, MutexGuard, Ordering};
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::ptr::NonNull;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::{fmt, thread};

pub struct WriteHandle<T, O>
where
    T: Absorb<O>,
{
    epochs: crate::Epochs,
    w_handle: NonNull<T>,
    oplog: VecDeque<O>,
    swap_index: usize,
    r_handle: ReadHandle<T>,
    last_epochs: Vec<usize>,
    #[cfg(test)]
    refreshes: usize,
    #[cfg(test)]
    is_waiting: Arc<AtomicBool>,
    first: bool,
    second: bool,
    taken: bool,
}
unsafe impl<T, O> Send for WriteHandle<T, O>
where
    T: Absorb<O>,
    T: Send,
    O: Send,
    ReadHandle<T>: Send,
{
}

impl<T, O> fmt::Debug for WriteHandle<T, O>
where
    T: Absorb<O> + fmt::Debug,
    O: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriteHandle")
            .field("epochs", &self.epochs)
            .field("w_handle", &self.w_handle)
            .field("oplog", &self.oplog)
            .field("swap_index", &self.swap_index)
            .field("r_handle", &self.r_handle)
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

pub struct Taken<T: Absorb<O>, O> {
    inner: Option<Box<T>>,
    _marker: PhantomData<O>,
}

impl<T: Absorb<O> + std::fmt::Debug, O> std::fmt::Debug for Taken<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Taken")
            .field(
                "inner",
                self.inner
                    .as_ref()
                    .expect("inner is only taken in `into_box` which drops self"),
            )
            .finish()
    }
}

impl<T: Absorb<O>, O> Deref for Taken<T, O> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
            .as_ref()
            .expect("inner is only taken in `into_box` which drops self")
    }
}

impl<T: Absorb<O>, O> DerefMut for Taken<T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .as_mut()
            .expect("inner is only taken in `into_box` which drops self")
    }
}

impl<T: Absorb<O>, O> Taken<T, O> {
    pub unsafe fn into_box(mut self) -> Box<T> {
        self.inner
            .take()
            .expect("inner is only taken here then self is dropped")
    }
}

impl<T: Absorb<O>, O> Drop for Taken<T, O> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            T::drop_second(inner);
        }
    }
}

impl<T, O> WriteHandle<T, O>
where
    T: Absorb<O>,
{
    fn take_inner(&mut self) -> Option<Taken<T, O>> {
        use std::ptr;
        
        if self.taken {
            return None;
        }

        self.taken = true;

        if self.first || !self.oplog.is_empty() {
            self.publish();
        }
        if !self.oplog.is_empty() {
            self.publish();
        }
        assert!(self.oplog.is_empty());

        let r_handle = self.r_handle.inner.swap(ptr::null_mut(), Ordering::Release);

        let epochs = Arc::clone(&self.epochs);
        let mut epochs = epochs.lock().unwrap();
        self.wait(&mut epochs);
        fence(Ordering::SeqCst);
        
        Absorb::drop_first(unsafe { Box::from_raw(self.w_handle.as_ptr()) });
        
        let boxed_r_handle = unsafe { Box::from_raw(r_handle) };

        Some(Taken {
            inner: Some(boxed_r_handle),
            _marker: PhantomData,
        })
    }
}

impl<T, O> Drop for WriteHandle<T, O>
where
    T: Absorb<O>,
{
    fn drop(&mut self) {
        if let Some(inner) = self.take_inner() {
            drop(inner);
        }
    }
}

impl<T, O> WriteHandle<T, O>
where
    T: Absorb<O>,
{
    pub(crate) fn new(w_handle: T, epochs: crate::Epochs, r_handle: ReadHandle<T>) -> Self {
        Self {
            epochs,
            
            w_handle: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(w_handle))) },
            oplog: VecDeque::new(),
            swap_index: 0,
            r_handle,
            last_epochs: Vec::new(),
            #[cfg(test)]
            is_waiting: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            refreshes: 0,
            first: true,
            second: true,
            taken: false,
        }
    }

    fn wait(&mut self, epochs: &mut MutexGuard<'_, slab::Slab<Arc<AtomicUsize>>>) {
        let mut iter = 0;
        let mut starti = 0;

        #[cfg(test)]
        {
            self.is_waiting.store(true, Ordering::Relaxed);
        }
        
        self.last_epochs.resize(epochs.capacity(), 0);
        'retry: loop {
            
            for (ii, (ri, epoch)) in epochs.iter().enumerate().skip(starti) {
                
                if self.last_epochs[ri] % 2 == 0 {
                    continue;
                }

                let now = epoch.load(Ordering::Acquire);
                if now != self.last_epochs[ri] {
                    
                } else {
                    
                    starti = ii;

                    if !cfg!(loom) {
                        
                        if iter != 20 {
                            iter += 1;
                        } else {
                            thread::yield_now();
                        }
                    }

                    #[cfg(loom)]
                    loom::thread::yield_now();

                    continue 'retry;
                }
            }
            break;
        }
        #[cfg(test)]
        {
            self.is_waiting.store(false, Ordering::Relaxed);
        }
    }

    
    pub fn publish(&mut self) -> &mut Self {
        
        let epochs = Arc::clone(&self.epochs);
        let mut epochs = epochs.lock().unwrap();

        self.wait(&mut epochs);

        if !self.first {
            
            let w_handle = unsafe { self.w_handle.as_mut() };
            
            let r_handle = unsafe {
                self.r_handle
                    .inner
                    .load(Ordering::Acquire)
                    .as_ref()
                    .unwrap()
            };

            if self.second {
                Absorb::sync_with(w_handle, r_handle);
                self.second = false
            }
            
            if self.swap_index != 0 {
                for op in self.oplog.drain(0..self.swap_index) {
                    T::absorb_second(w_handle, op, r_handle);
                }
            }
            
            for op in self.oplog.iter_mut() {
                T::absorb_first(w_handle, op, r_handle);
            }
            
            self.swap_index = self.oplog.len();
            
        } else {
            self.first = false
        }
        
        let r_handle = self
            .r_handle
            .inner
            .swap(self.w_handle.as_ptr(), Ordering::Release);

            
        self.w_handle = unsafe { NonNull::new_unchecked(r_handle) };
        
        fence(Ordering::SeqCst);

        for (ri, epoch) in epochs.iter() {
            self.last_epochs[ri] = epoch.load(Ordering::Acquire);
        }

        #[cfg(test)]
        {
            self.refreshes += 1;
        }

        self
    }
    
    pub fn flush(&mut self) {
        if self.has_pending_operations() {
            self.publish();
        }
    }

    
    pub fn has_pending_operations(&self) -> bool {
        
        self.swap_index < self.oplog.len()
    }

    
    pub fn append(&mut self, op: O) -> &mut Self {
        self.extend(std::iter::once(op));
        self
    }
    
    pub fn raw_write_handle(&mut self) -> NonNull<T> {
        self.w_handle
    }
    
    pub fn take(mut self) -> Taken<T, O> {
        
        self.take_inner()
            .expect("inner is only taken here then self is dropped")
    }
}

use std::ops::Deref;
impl<T, O> Deref for WriteHandle<T, O>
where
    T: Absorb<O>,
{
    type Target = ReadHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.r_handle
    }
}

impl<T, O> Extend<O> for WriteHandle<T, O>
where
    T: Absorb<O>,
{
    
    fn extend<I>(&mut self, ops: I)
    where
        I: IntoIterator<Item = O>,
    {
        if self.first {
            
            let mut w_inner = self.raw_write_handle();
            let w_inner = unsafe { w_inner.as_mut() };
            let r_handle = self.enter().expect("map has not yet been destroyed");
            
            for op in ops {
                Absorb::absorb_second(w_inner, op, &*r_handle);
            }
        } else {
            self.oplog.extend(ops);
        }
    }
}


#[allow(dead_code)]
struct CheckWriteHandleSend;

#[cfg(test)]
mod tests {
    use crate::sync::{AtomicUsize, Mutex, Ordering};
    use crate::Absorb;
    use slab::Slab;
    include!("./utilities.rs");

    #[test]
    fn append_test() {
        let (mut w, _r) = crate::new::<i32, _>();
        assert_eq!(w.first, true);
        w.append(CounterAddOp(1));
        assert_eq!(w.oplog.len(), 0);
        assert_eq!(w.first, true);
        w.publish();
        assert_eq!(w.first, false);
        w.append(CounterAddOp(2));
        w.append(CounterAddOp(3));
        assert_eq!(w.oplog.len(), 2);
    }

    #[test]
    fn take_test() {
        
        let (mut w, _r) = crate::new_from_empty::<i32, _>(2);
        w.append(CounterAddOp(1));
        w.publish();
        w.append(CounterAddOp(1));
        w.publish();
        assert_eq!(*w.take(), 4);

        
        let (mut w, _r) = crate::new_from_empty::<i32, _>(2);
        w.append(CounterAddOp(1));
        w.publish();
        w.append(CounterAddOp(1));
        w.publish();
        w.append(CounterAddOp(2));
        assert_eq!(*w.take(), 6);

        
        let (mut w, _r) = crate::new_from_empty::<i32, _>(2);
        w.append(CounterAddOp(1));
        w.publish();
        w.append(CounterAddOp(1));
        assert_eq!(*w.take(), 4);
        
        let (mut w, _r) = crate::new_from_empty::<i32, _>(2);
        w.append(CounterAddOp(1));
        assert_eq!(*w.take(), 3);

        
        let (mut w, _r) = crate::new_from_empty::<i32, _>(2);
        w.append(CounterAddOp(1));
        w.publish();
        assert_eq!(*w.take(), 3);

        
        let (w, _r) = crate::new_from_empty::<i32, _>(2);
        assert_eq!(*w.take(), 2);
    }

    #[test]
    fn wait_test() {
        use std::sync::{Arc, Barrier};
        use std::thread;
        let (mut w, _r) = crate::new::<i32, _>();
        
        let test_epochs: crate::Epochs = Default::default();
        let mut test_epochs = test_epochs.lock().unwrap();
        
        w.wait(&mut test_epochs);
        
        let held_epoch = Arc::new(AtomicUsize::new(1));

        w.last_epochs = vec![2, 2, 1];
        let mut epochs_slab = Slab::new();
        epochs_slab.insert(Arc::new(AtomicUsize::new(2)));
        epochs_slab.insert(Arc::new(AtomicUsize::new(2)));
        epochs_slab.insert(Arc::clone(&held_epoch));

        let barrier = Arc::new(Barrier::new(2));

        let is_waiting = Arc::clone(&w.is_waiting);
        
        let is_waiting_v = is_waiting.load(Ordering::Relaxed);
        assert_eq!(false, is_waiting_v);

        let barrier2 = Arc::clone(&barrier);
        let test_epochs = Arc::new(Mutex::new(epochs_slab));
        let wait_handle = thread::spawn(move || {
            barrier2.wait();
            let mut test_epochs = test_epochs.lock().unwrap();
            w.wait(&mut test_epochs);
        });

        barrier.wait();
        
        while !is_waiting.load(Ordering::Relaxed) {
            thread::yield_now();
        }

        held_epoch.fetch_add(1, Ordering::SeqCst);

        
        let _ = wait_handle.join();
    }

    #[test]
    fn flush_noblock() {
        let (mut w, r) = crate::new::<i32, _>();
        w.append(CounterAddOp(42));
        w.publish();
        assert_eq!(*r.enter().unwrap(), 42);

        
        let _count = r.enter();
        
        assert_eq!(w.oplog.iter().skip(w.swap_index).count(), 0);
        assert!(!w.has_pending_operations());
    }

    #[test]
    fn flush_no_refresh() {
        let (mut w, _) = crate::new::<i32, _>();
        
        assert!(!w.has_pending_operations());
        w.publish();
        assert!(!w.has_pending_operations());
        assert_eq!(w.refreshes, 1);

        w.append(CounterAddOp(42));
        assert!(w.has_pending_operations());
        w.publish();
        assert!(!w.has_pending_operations());
        assert_eq!(w.refreshes, 2);

        w.append(CounterAddOp(42));
        assert!(w.has_pending_operations());
        w.publish();
        assert!(!w.has_pending_operations());
        assert_eq!(w.refreshes, 3);

        
        assert!(!w.has_pending_operations());
        w.publish();
        assert_eq!(w.refreshes, 4);
    }
}
