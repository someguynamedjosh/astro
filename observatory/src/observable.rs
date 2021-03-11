use crate::{
    observer::{ObserverInternalFns, ObserverList},
    static_state,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::{Rc, Weak},
};

pub(crate) trait ObservableInternalFns {
    fn add_observer(&self, observer: Weak<dyn ObserverInternalFns>);
    fn remove_observer(&self, observer: &Weak<dyn ObserverInternalFns>);
    fn get_unique_data_address(&self) -> *const ();
}

#[repr(C)]
struct ObservableData<T: ?Sized> {
    observers: ObserverList,
    value: RefCell<T>,
}

impl<T: ?Sized> ObservableData<T> {
    fn after_modified(&self) {
        self.observers.broadcast_stale();
        self.observers.broadcast_ready(true);
    }
}

impl<T: PartialEq> ObservableInternalFns for ObservableData<T> {
    fn add_observer(&self, observer: Weak<dyn ObserverInternalFns>) {
        self.observers.add(observer);
    }

    fn remove_observer(&self, observer: &Weak<dyn ObserverInternalFns>) {
        self.observers.remove(observer);
    }

    fn get_unique_data_address(&self) -> *const () {
        self.value.as_ptr() as _
    }
}

pub struct ObservablePtr<T: ?Sized + PartialEq + 'static> {
    ptr: Rc<ObservableData<T>>,
}

impl<T: ?Sized + PartialEq + 'static> Clone for ObservablePtr<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: Rc::clone(&self.ptr),
        }
    }
}

pub struct ObservableRef<'a, T: ?Sized + PartialEq + 'a> {
    raw: Ref<'a, T>,
}

impl<'a, T: ?Sized + PartialEq + 'a> From<Ref<'a, T>> for ObservableRef<'a, T> {
    fn from(raw: Ref<'a, T>) -> Self {
        Self { raw }
    }
}

impl<'a, T: ?Sized + PartialEq + 'a> Deref for ObservableRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &*self.raw
    }
}

pub struct ObservableRefMut<'a, T: ?Sized + PartialEq + 'a> {
    data: Rc<ObservableData<T>>,
    raw: Option<RefMut<'a, T>>,
}

impl<'a, T: ?Sized + PartialEq + 'a> Deref for ObservableRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.raw.as_deref().unwrap()
    }
}

impl<'a, T: ?Sized + PartialEq + 'a> DerefMut for ObservableRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.raw.as_deref_mut().unwrap()
    }
}
impl<'a, T: ?Sized + PartialEq + 'a> Drop for ObservableRefMut<'a, T> {
    fn drop(&mut self) {
        // Drop the reference so that observers notified of the changes can read the new data.
        self.raw = None;
        self.data.after_modified();
    }
}

impl<T: PartialEq + 'static> ObservablePtr<T> {
    pub fn new(value: T) -> Self {
        let bx = ObservableData {
            observers: Default::default(),
            value: RefCell::new(value),
        };
        let ptr = Rc::new(bx);
        Self { ptr }
    }

    pub fn borrow(&self) -> ObservableRef<T> {
        static_state::note_observed(Rc::clone(&self.ptr) as _);
        From::from(self.ptr.value.borrow())
    }

    pub fn borrow_untracked(&self) -> ObservableRef<T> {
        From::from(self.ptr.value.borrow())
    }

    pub fn borrow_mut(&self) -> ObservableRefMut<T> {
        ObservableRefMut {
            data: Rc::clone(&self.ptr),
            raw: Some(self.ptr.value.borrow_mut()),
        }
    }

    pub fn set(&self, new_value: T) {
        let mut value_storage = self.ptr.value.borrow_mut();
        if new_value != *value_storage {
            *value_storage = new_value;
        }
        drop(value_storage);
        self.ptr.after_modified();
    }
}
