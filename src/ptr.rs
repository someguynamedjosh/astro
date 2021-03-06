use std::{
    alloc::Layout,
    cell::Cell,
    marker::Unsize,
    mem::MaybeUninit,
    ops::{CoerceUnsized, Deref},
    ptr::NonNull,
};

pub(crate) struct PtrTarget<T: ?Sized> {
    #[cfg(debug_assertions)]
    weak_count: Cell<usize>,
    ref_count: Cell<usize>,
    value: T,
}

impl<T> PtrTarget<T> {
    #[cfg(not(debug_assertions))]
    fn new(value: T) -> Self {
        Self {
            ref_count: Cell::new(1),
            value,
        }
    }

    #[cfg(debug_assertions)]
    fn new(value: T) -> Self {
        Self {
            weak_count: Cell::new(1),
            ref_count: Cell::new(1),
            value,
        }
    }

    #[cfg(not(debug_assertions))]
    fn new_cyclic(value: T) -> Self {
        Self {
            ref_count: Cell::new(0),
            value,
        }
    }

    #[cfg(debug_assertions)]
    fn new_cyclic(value: T) -> Self {
        Self {
            weak_count: Cell::new(1),
            ref_count: Cell::new(0),
            value,
        }
    }
}

impl<T: ?Sized> PtrTarget<T> {
    #[cfg(not(debug_assertions))]
    fn inc_weak(&self) {}

    #[cfg(debug_assertions)]
    fn inc_weak(&self) {
        let rc = &self.weak_count;
        rc.set(rc.get() + 1);
    }

    #[cfg(not(debug_assertions))]
    fn assert_not_dropped(&self) {}

    #[cfg(debug_assertions)]
    fn assert_not_dropped(&self) {
        if self.ref_count.get() == 0 {
            panic!("Tried to access a ThinPtr's target after it was dropped or before it was initialized!");
        }
    }
}

pub(crate) struct ThinPtr<T: ?Sized>(NonNull<PtrTarget<T>>);

impl<T> ThinPtr<T> {
    pub(crate) fn new(value: T) -> Self {
        let target = PtrTarget::new(value);
        let ptr = Box::leak(Box::new(target)).into();
        Self(ptr)
    }

    pub(crate) fn new_cyclic(builder: impl FnOnce(&WeakThinPtr<T>) -> T) -> Self {
        let uninit_target = PtrTarget::new_cyclic(MaybeUninit::<T>::uninit());
        let uninit_ptr: NonNull<_> = Box::leak(Box::new(uninit_target)).into();
        let init_ptr = uninit_ptr.cast();
        let init_weak_ptr = WeakThinPtr(init_ptr);
        let value = builder(&init_weak_ptr);
        unsafe {
            let value_target = std::ptr::addr_of_mut!((*init_ptr.as_ptr()).value);
            // MaybeUninit<T> has the same layout as T
            std::ptr::write(value_target, value);

            let old = init_ptr.as_ref().ref_count.replace(1);
            debug_assert_eq!(old, 0);
            init_ptr.as_ref().inc_weak();
        }
        drop(init_weak_ptr);
        Self(init_ptr)
    }
}

impl<T: ?Sized> ThinPtr<T> {
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }

    pub(crate) fn downgrade(&self) -> WeakThinPtr<T> {
        unsafe {
            self.0.as_ref().inc_weak();
        }
        WeakThinPtr(self.0)
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<ThinPtr<U>> for ThinPtr<T> {}

impl<T: ?Sized> Deref for ThinPtr<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref().assert_not_dropped() };
        &unsafe { self.0.as_ref() }.value
    }
}

#[cfg(debug_assertions)]
impl<T: ?Sized> Clone for ThinPtr<T> {
    fn clone(&self) -> Self {
        unsafe {
            let rc = &self.0.as_ref().ref_count;
            rc.set(rc.get() + 1);
            let wc = &self.0.as_ref().weak_count;
            wc.set(wc.get() + 1);
        }
        Self(self.0)
    }
}

#[cfg(not(debug_assertions))]
impl<T: ?Sized> Clone for ThinPtr<T> {
    fn clone(&self) -> Self {
        unsafe {
            let rc = &self.0.as_ref().ref_count;
            rc.set(rc.get() + 1);
        }
        Self(self.0)
    }
}

// Just drop the value and keep tracking ptr counts if we are using debug assertions.
#[cfg(debug_assertions)]
impl<T: ?Sized> Drop for ThinPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let rc = &self.0.as_ref().ref_count;
            let refs = rc.get() - 1;
            rc.set(refs);
            // If there are no more ThinPtrs...
            if refs == 0 {
                let value_ptr = &mut (*self.0.as_ptr()).value;
                std::ptr::drop_in_place(value_ptr);
            }
            let wc = &self.0.as_ref().weak_count;
            let weak_refs = wc.get() - 1;
            wc.set(weak_refs);
            // If there are no more ThinPtrs and WeakThinPtrs...
            if weak_refs == 0 {
                let layout = Layout::for_value(self.0.as_ref());
                std::alloc::dealloc(self.0.as_ptr() as _, layout);
            }
        }
    }
}

// Drop everything at once if we are not using debug assertions.
#[cfg(not(debug_assertions))]
impl<T: ?Sized> Drop for ThinPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let rc = &self.0.as_ref().ref_count;
            let val = rc.get() - 1;
            rc.set(val);
            if val == 0 {
                let layout = Layout::for_value(self.0.as_ref());
                let value_ptr = std::ptr::addr_of_mut!((*self.0.as_ptr()).value);
                std::ptr::drop_in_place(value_ptr);
                std::alloc::dealloc(self.0.as_ptr() as _, layout);
            }
        }
    }
}

pub(crate) struct WeakThinPtr<T: ?Sized>(NonNull<PtrTarget<T>>);

impl<T: ?Sized> WeakThinPtr<T> {
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }

    /// You must guarantee that there is at least one ThinPtr pointing to the same object to make
    /// this safe. Otherwise the data will have been dropped so the returned reference will point
    /// to uninitialized memory.
    pub(crate) unsafe fn deref(&self) -> &T {
        self.0.as_ref().assert_not_dropped();
        &self.0.as_ref().value
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<WeakThinPtr<U>> for WeakThinPtr<T> {}

#[cfg(debug_assertions)]
impl<T: ?Sized> Clone for WeakThinPtr<T> {
    fn clone(&self) -> Self {
        unsafe {
            let rc = &self.0.as_ref().weak_count;
            rc.set(rc.get() + 1);
        }
        Self(self.0)
    }
}

#[cfg(not(debug_assertions))]
impl<T: ?Sized> Clone for WeakThinPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

// Only deallocate the memory we are pointing to once all weak refs are gone, but only if we are
// using debug assertions. (Otherwise do nothing because memory is deallocated once all strong
// refs are gone.)
#[cfg(debug_assertions)]
impl<T: ?Sized> Drop for WeakThinPtr<T> {
    fn drop(&mut self) {
        unsafe {
            let rc = &(*self.0.as_ptr()).weak_count;
            let val = rc.get() - 1;
            rc.set(val);
            if val == 0 && (*self.0.as_ptr()).ref_count.get() == 0 {
                let layout = Layout::for_value(self.0.as_ref());
                std::alloc::dealloc(self.0.as_ptr() as _, layout);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_and_drop_cyclic() {
        struct CyclicHolder(WeakThinPtr<CyclicHolder>);
        let ptr = ThinPtr::new_cyclic(|self_ref| CyclicHolder(WeakThinPtr::clone(self_ref)));
        drop(ptr);
    }

    #[test]
    fn two_strong() {
        let ptr = ThinPtr::new(123);
        let ptr2 = ThinPtr::clone(&ptr);
        drop(ptr);
        assert_eq!(*ptr2, 123);
        drop(ptr2);
    }

    #[test]
    fn many_strong() {
        let ptr = ThinPtr::new(123);
        let ptr2 = ThinPtr::clone(&ptr);
        let ptr3 = ThinPtr::clone(&ptr);
        let ptr4 = ThinPtr::clone(&ptr);
        drop(ptr2);
        drop(ptr);
        drop(ptr3);
        assert_eq!(*ptr4, 123);
        drop(ptr4);
    }

    #[test]
    fn one_strong_many_weak() {
        let ptr = ThinPtr::new(123);
        let weak1 = ptr.downgrade();
        let weak2 = ptr.downgrade();
        let weak3 = ptr.downgrade();
        assert_eq!(*unsafe { weak2.deref() }, 123);
        drop(weak3);
        drop(weak2);
        assert_eq!(*unsafe { weak1.deref() }, 123);
        drop(ptr);
        drop(weak1);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn debug_assert() {
        let ptr = ThinPtr::new(123);
        let weak = ptr.downgrade();
        drop(ptr);
        unsafe { weak.deref() };
    }
}
