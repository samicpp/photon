
use std::{mem::ManuallyDrop, ptr, sync::{Arc, atomic::{AtomicPtr, Ordering}}};

#[derive(Debug)]
pub struct AtomicArc<T>(pub AtomicPtr<T>);

impl<T> AtomicArc<T> {
    pub const fn empty() -> Self {
        Self(AtomicPtr::new(ptr::null_mut()))
    }
    pub fn new(value: T) -> Self {
        Self(AtomicPtr::new(Arc::into_raw(Arc::new(value)) as *mut _))
    }
    pub fn load(&self) -> Option<Arc<T>> {
        unsafe {
            let ptr = self.0.load(Ordering::Acquire);
            
            if ptr.is_null() {
                None
            }
            else {
                Some(
                    Arc::clone(&ManuallyDrop::new(Arc::from_raw(ptr)))
                )
            }
        }
    }
    pub unsafe fn load_unchecked(&self) -> Arc<T> {
        unsafe {
            let ptr = self.0.load(Ordering::Acquire);
            Arc::clone(&ManuallyDrop::new(Arc::from_raw(ptr)))
        }
    }
    pub fn store(&self, arc: Option<Arc<T>>) {
        let ptr = self.0.swap(
            match arc {
                Some(arc) => Arc::into_raw(arc) as *mut T,
                None => ptr::null_mut(),
            }, 
            Ordering::AcqRel
        );
        if !ptr.is_null() { 
            let _ = unsafe { drop(Arc::from_raw(ptr)) };
        }
    }
    pub fn swap(&self, arc: Option<Arc<T>>) -> Option<Arc<T>> {
        let ptr = self.0.swap(
            match arc {
                Some(arc) => Arc::into_raw(arc) as *mut T,
                None => ptr::null_mut(),
            }, 
            Ordering::AcqRel
        );
        if ptr.is_null() {
            None
        }
        else {
            unsafe { Some(Arc::from_raw(ptr)) }
        }
    }
}

impl<T> Drop for AtomicArc<T> {
    fn drop(&mut self) {
        let ptr = *self.0.get_mut();
        if !ptr.is_null() {
            let _ = unsafe { drop(Arc::from_raw(ptr)) };
        }
    }
}

unsafe impl<T: Send + Sync> Send for AtomicArc<T> { }
unsafe impl<T: Sync + Send> Sync for AtomicArc<T> { }
