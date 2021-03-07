use crate::{
    observer::{ObserverInternalFns, ObserverList},
    ptr::{ThinPtr, WeakThinPtr},
    static_state,
};
use std::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub(crate) trait ObservableInternalFns {
    fn add_observer(&self, observer: WeakThinPtr<dyn ObserverInternalFns>);
    fn remove_observer(&self, observer: &WeakThinPtr<dyn ObserverInternalFns>);
}

#[repr(C)]
struct ObservableData<T: ?Sized> {
    observers: ObserverList,
    immutable_refs: Cell<usize>,
    mutable_ref: Cell<bool>,
    value: UnsafeCell<T>,
}

impl<T: ?Sized> ObservableData<T> {
    fn after_modified(&self) {
        self.observers.broadcast_stale();
        self.observers.broadcast_ready(true);
    }
}

impl<T: PartialEq> ObservableInternalFns for ObservableData<T> {
    fn add_observer(&self, observer: WeakThinPtr<dyn ObserverInternalFns>) {
        self.observers.add(observer);
    }

    fn remove_observer(&self, observer: &WeakThinPtr<dyn ObserverInternalFns>) {
        self.observers.remove(observer);
    }
}

#[derive(Clone)]
pub struct ObservablePtr<T: ?Sized + PartialEq + 'static> {
    ptr: ThinPtr<ObservableData<T>>,
}

#[derive(Clone)]
pub struct ObservableRef<'a, T: ?Sized + PartialEq + 'static> {
    ptr: NonNull<ObservableData<T>>,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a, T: ?Sized + PartialEq + 'static> From<&ThinPtr<ObservableData<T>>>
    for ObservableRef<'a, T>
{
    fn from(ptr: &ThinPtr<ObservableData<T>>) -> Self {
        Self {
            ptr: ptr.get_raw_ptr(),
            _lifetime: PhantomData,
        }
    }
}

impl<'a, T: ?Sized + PartialEq + 'static> Deref for ObservableRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        let value_ptr = unsafe { self.ptr.as_ref() }.value.get();
        unsafe { &*value_ptr }
    }
}

impl<'a, T: ?Sized + PartialEq + 'static> Drop for ObservableRef<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let data = self.ptr.as_ref();
            data.immutable_refs.set(data.immutable_refs.get() - 1);
        }
    }
}

#[derive(Clone)]
pub struct ObservableRefMut<'a, T: ?Sized + PartialEq + 'static> {
    ptr: NonNull<ObservableData<T>>,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a, T: ?Sized + PartialEq + 'static> From<&ThinPtr<ObservableData<T>>>
    for ObservableRefMut<'a, T>
{
    fn from(ptr: &ThinPtr<ObservableData<T>>) -> Self {
        Self {
            ptr: ptr.get_raw_ptr(),
            _lifetime: PhantomData,
        }
    }
}

impl<'a, T: ?Sized + PartialEq + 'static> Deref for ObservableRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        let value_ptr = unsafe { self.ptr.as_ref() }.value.get();
        unsafe { &*value_ptr }
    }
}

impl<'a, T: ?Sized + PartialEq + 'static> DerefMut for ObservableRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        let value_ptr = unsafe { self.ptr.as_ref() }.value.get();
        unsafe { &mut *value_ptr }
    }
}

impl<'a, T: ?Sized + PartialEq + 'static> Drop for ObservableRefMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let data = self.ptr.as_ref();
            data.mutable_ref.set(false);
            data.after_modified();
        }
    }
}

impl<T: PartialEq + 'static> ObservablePtr<T> {
    pub fn new(value: T) -> Self {
        let bx = ObservableData {
            observers: Default::default(),
            immutable_refs: Cell::new(0),
            mutable_ref: Cell::new(false),
            value: UnsafeCell::new(value),
        };
        let ptr = ThinPtr::new(bx);
        Self { ptr }
    }

    pub fn borrow(&self) -> ObservableRef<T> {
        if self.ptr.mutable_ref.get() {
            panic!("Cannot borrow immutably while also borrowed mutably!");
        }
        self.ptr
            .immutable_refs
            .set(self.ptr.immutable_refs.get() + 1);
        static_state::note_observed(ThinPtr::clone(&self.ptr) as _);
        From::from(&self.ptr)
    }

    pub fn borrow_untracked(&self) -> ObservableRef<T> {
        if self.ptr.mutable_ref.get() {
            panic!("Cannot borrow immutably while also borrowed mutably!");
        }
        self.ptr
            .immutable_refs
            .set(self.ptr.immutable_refs.get() + 1);
        From::from(&self.ptr)
    }

    fn reserve_mut_borrow(&self) {
        if self.ptr.immutable_refs.get() > 0 {
            panic!("Cannot borrow mutably when already borrowed immutably!");
        }
        if self.ptr.mutable_ref.get() {
            panic!("Cannot borrow mutably more than once!");
        }
        self.ptr.mutable_ref.set(true);
    }

    pub fn borrow_mut(&self) -> ObservableRefMut<T> {
        self.reserve_mut_borrow();
        From::from(&self.ptr)
    }

    pub fn set(&self, new_value: T) {
        self.reserve_mut_borrow();
        let value_storage = unsafe { &mut *self.ptr.value.get() };
        if new_value != *value_storage {
            *value_storage = new_value;
        }
        self.ptr.mutable_ref.set(false);
        self.ptr.after_modified();
    }
}
