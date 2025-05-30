use crate::sync::{AtomicUsize, Ordering};
use std::cell::Cell;
use std::mem;

#[derive(Debug, Copy, Clone)]
pub(super) struct ReadHandleState<'rh> {
    pub(super) epoch: &'rh AtomicUsize,
    pub(super) enters: &'rh Cell<usize>,
}

impl<'rh, T> From<&'rh super::ReadHandle<T>> for ReadHandleState<'rh> {
    fn from(rh: &'rh super::ReadHandle<T>) -> Self {
        Self {
            epoch: &rh.epoch,
            enters: &rh.enters,
        }
    }
}

#[derive(Debug)]
pub struct ReadGuard<'rh, T: ?Sized> {
    
    pub(super) t: &'rh T,
    pub(super) handle: ReadHandleState<'rh>,
}

impl<'rh, T: ?Sized> ReadGuard<'rh, T> {
    pub fn map<F, U: ?Sized>(orig: Self, f: F) -> ReadGuard<'rh, U>
    where
        F: for<'a> FnOnce(&'a T) -> &'a U,
    {
        let rg = ReadGuard {
            t: f(orig.t),
            handle: orig.handle,
        };
        mem::forget(orig);
        rg
    }

    pub fn try_map<F, U: ?Sized>(orig: Self, f: F) -> Option<ReadGuard<'rh, U>>
    where
        F: for<'a> FnOnce(&'a T) -> Option<&'a U>,
    {
        let rg = ReadGuard {
            t: f(orig.t)?,
            handle: orig.handle,
        };
        mem::forget(orig);
        Some(rg)
    }
}

impl<'rh, T: ?Sized> AsRef<T> for ReadGuard<'rh, T> {
    fn as_ref(&self) -> &T {
        self.t
    }
}

impl<'rh, T: ?Sized> std::ops::Deref for ReadGuard<'rh, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.t
    }
}

impl<'rh, T: ?Sized> Drop for ReadGuard<'rh, T> {
    fn drop(&mut self) {
        let enters = self.handle.enters.get() - 1;
        self.handle.enters.set(enters);
        if enters == 0 {
            
            self.handle.epoch.fetch_add(1, Ordering::AcqRel);
        }
    }
}
