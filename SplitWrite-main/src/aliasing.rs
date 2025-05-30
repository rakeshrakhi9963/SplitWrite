
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;

#[allow(unused_imports)]
use crate::Absorb;

pub trait DropBehavior {
  
    const DO_DROP: bool;
}

#[repr(transparent)]
pub struct Aliased<T, D>
where
    D: DropBehavior,
{
    aliased: MaybeUninit<T>,

    drop_behavior: PhantomData<D>,

    _no_auto_send: PhantomData<*const T>,
}

impl<T, D> Aliased<T, D>
where
    D: DropBehavior,
{
    pub unsafe fn alias(&self) -> Self {
      
        Aliased {
            aliased: std::ptr::read(&self.aliased),
            drop_behavior: PhantomData,
            _no_auto_send: PhantomData,
        }
    }
    pub fn from(t: T) -> Self {
        Self {
            aliased: MaybeUninit::new(t),
            drop_behavior: PhantomData,
            _no_auto_send: PhantomData,
        }
    }

    pub unsafe fn change_drop<D2: DropBehavior>(self) -> Aliased<T, D2> {
        Aliased {
            // safety:
            aliased: std::ptr::read(&self.aliased),
            drop_behavior: PhantomData,
            _no_auto_send: PhantomData,
        }
    }
}

unsafe impl<T, D> Send for Aliased<T, D>
where
    D: DropBehavior,
    T: Send + Sync,
{
}
unsafe impl<T, D> Sync for Aliased<T, D>
where
    D: DropBehavior,
    T: Sync,
{
}

impl<T, D> Drop for Aliased<T, D>
where
    D: DropBehavior,
{
    fn drop(&mut self) {
        if D::DO_DROP {
          
            unsafe { std::ptr::drop_in_place(self.aliased.as_mut_ptr()) }
        }
    }
}

impl<T, D> AsRef<T> for Aliased<T, D>
where
    D: DropBehavior,
{
    fn as_ref(&self) -> &T {
        
        unsafe { &*self.aliased.as_ptr() }
    }
}

impl<T, D> Deref for Aliased<T, D>
where
    D: DropBehavior,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

use std::hash::{Hash, Hasher};
impl<T, D> Hash for Aliased<T, D>
where
    D: DropBehavior,
    T: Hash,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.as_ref().hash(state)
    }
}

use std::fmt;
impl<T, D> fmt::Debug for Aliased<T, D>
where
    D: DropBehavior,
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl<T, D> PartialEq for Aliased<T, D>
where
    D: DropBehavior,
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(other.as_ref())
    }
}

impl<T, D> Eq for Aliased<T, D>
where
    D: DropBehavior,
    T: Eq,
{
}

impl<T, D> PartialOrd for Aliased<T, D>
where
    D: DropBehavior,
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }

    fn lt(&self, other: &Self) -> bool {
        self.as_ref().lt(other.as_ref())
    }

    fn le(&self, other: &Self) -> bool {
        self.as_ref().le(other.as_ref())
    }

    fn gt(&self, other: &Self) -> bool {
        self.as_ref().gt(other.as_ref())
    }

    fn ge(&self, other: &Self) -> bool {
        self.as_ref().ge(other.as_ref())
    }
}

impl<T, D> Ord for Aliased<T, D>
where
    D: DropBehavior,
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

use std::borrow::Borrow;
impl<T, D> Borrow<T> for Aliased<T, D>
where
    D: DropBehavior,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}
impl<D> Borrow<str> for Aliased<String, D>
where
    D: DropBehavior,
{
    fn borrow(&self) -> &str {
        self.as_ref()
    }
}
impl<D> Borrow<std::path::Path> for Aliased<std::path::PathBuf, D>
where
    D: DropBehavior,
{
    fn borrow(&self) -> &std::path::Path {
        self.as_ref()
    }
}
impl<T, D> Borrow<[T]> for Aliased<Vec<T>, D>
where
    D: DropBehavior,
{
    fn borrow(&self) -> &[T] {
        self.as_ref()
    }
}
impl<T, D> Borrow<T> for Aliased<Box<T>, D>
where
    T: ?Sized,
    D: DropBehavior,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}
impl<T, D> Borrow<T> for Aliased<std::sync::Arc<T>, D>
where
    T: ?Sized,
    D: DropBehavior,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}
impl<T, D> Borrow<T> for Aliased<std::rc::Rc<T>, D>
where
    T: ?Sized,
    D: DropBehavior,
{
    fn borrow(&self) -> &T {
        self.as_ref()
    }
}
