use crate::{observable::ObservableInternalFns, static_state};
use std::{
    cell::{Cell, Ref, RefCell},
    rc::{Rc, Weak},
};

pub(crate) trait ObserverInternalFns {
    fn send_stale(&self);
    fn send_ready(&self, changed: bool);
    fn update(&self);
    fn get_unique_data_address(&self) -> *const ();
}

/// Helper struct which stores observers that should be notified whenever an observable object
/// changes. Used by both Observable and Derivation.
#[derive(Default)]
pub(crate) struct ObserverList {
    observers: Cell<Vec<Weak<dyn ObserverInternalFns>>>,
}

impl ObserverList {
    pub fn broadcast_stale(&self) {
        let list = self.observers.take();
        for observer in &list {
            observer.upgrade().unwrap().send_stale();
        }
        self.observers.set(list);
    }

    pub fn broadcast_ready(&self, changed: bool) {
        let list = self.observers.take();
        for observer in &list {
            observer.upgrade().unwrap().send_ready(changed);
        }
        self.observers.set(list);
    }

    pub fn add(&self, observer: Weak<dyn ObserverInternalFns>) {
        let mut list = self.observers.take();
        if list.iter().any(|item| Weak::ptr_eq(&observer, item)) {
            panic!("Tried to subscribe the same observer twice.");
        }
        list.push(observer);
        self.observers.set(list);
    }

    pub fn remove(&self, observer: &Weak<dyn ObserverInternalFns>) {
        let mut list = self.observers.take();
        let index = list.iter().position(|item| Weak::ptr_eq(item, observer));
        let index = index.expect(
            "(Internal error) Tried to unsubscribe an observer that was already unsubscribed.",
        );
        list.remove(index);
        self.observers.set(list);
    }
}

#[repr(C)]
struct DerivationData<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    this_ptr: Weak<dyn ObserverInternalFns>,
    observers: ObserverList,
    observing: Cell<Vec<Rc<dyn ObservableInternalFns>>>,
    num_stale_notifications: Cell<usize>,
    /// True if fields we are observing have changed and we need to update once
    /// num_stale_notifications reaches zero.
    should_update: Cell<bool>,
    compute_value: RefCell<F>,
    value: RefCell<T>,
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> ObserverInternalFns
    for DerivationData<T, F>
{
    /// Called when a value this observer depends on becomes stale.
    fn send_stale(&self) {
        let old = self
            .num_stale_notifications
            .replace(self.num_stale_notifications.get() + 1);
        // Don't send multiple stale notifications when we receive multiple stale notifications.
        if old == 0 {
            self.observers.broadcast_stale();
        }
    }

    /// Called when a value this observer depends on finishes updating. `changed` is false if the
    /// value has not changed.
    fn send_ready(&self, changed: bool) {
        let nsn = self.num_stale_notifications.get() - 1;
        self.num_stale_notifications.set(nsn);
        let should_update = self.should_update.get() || changed;
        self.should_update.set(should_update);
        if nsn == 0 {
            if should_update {
                self.update();
            } else {
                self.observers.broadcast_ready(false);
            }
        }
    }

    fn update(&self) {
        assert!(self.should_update.get());
        self.should_update.set(false);

        static_state::push_observing_stack();
        let new_value = (self.compute_value.borrow_mut())();
        let now_observing = static_state::pop_observing_stack();
        let was_observing = self.observing.take();
        for observable in &was_observing {
            let uda = observable.get_unique_data_address();
            // If we are no longer observing something we used to...
            if !now_observing
                .iter()
                .any(|other| uda == other.get_unique_data_address())
            {
                observable.remove_observer(&self.this_ptr)
            }
        }
        for observable in &now_observing {
            let uda = observable.get_unique_data_address();
            // If we are observing something we weren't observing before...
            if !was_observing
                .iter()
                .any(|other| uda == other.get_unique_data_address())
            {
                observable.add_observer(Weak::clone(&self.this_ptr));
            }
        }
        self.observing.set(now_observing);

        let changed = new_value != *self.value.borrow();
        if changed {
            self.value.replace(new_value);
        }

        self.observers.broadcast_ready(changed);
    }

    fn get_unique_data_address(&self) -> *const () {
        self.value.as_ptr() as _
    }
}

impl<T: PartialEq, F: FnMut() -> T> Drop for DerivationData<T, F> {
    fn drop(&mut self) {
        for observable in self.observing.take() {
            observable.remove_observer(&self.this_ptr);
        }
    }
}

impl<T: PartialEq, F: FnMut() -> T> ObservableInternalFns for DerivationData<T, F> {
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

pub struct DerivationPtr<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    ptr: Rc<DerivationData<T, F>>,
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> Clone for DerivationPtr<T, F> {
    fn clone(&self) -> Self {
        Self {
            ptr: Rc::clone(&self.ptr),
        }
    }
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> DerivationPtr<T, F> {
    pub fn new(mut compute_value: F) -> Self {
        static_state::push_observing_stack();
        let initial_value = compute_value();
        let observing = static_state::pop_observing_stack();
        let ptr = Rc::new_cyclic(|weak| DerivationData {
            this_ptr: Weak::clone(weak) as _,
            num_stale_notifications: Cell::new(0),
            observers: Default::default(),
            observing: Cell::new(observing.clone()),
            should_update: Cell::new(false),
            compute_value: RefCell::new(compute_value),
            value: RefCell::new(initial_value),
        });
        let weak = &ptr.this_ptr;
        for observable in &observing {
            observable.add_observer(Weak::clone(weak) as _);
        }
        Self { ptr }
    }

    pub fn new_dyn(compute_value: F) -> DerivationPtr<T, Box<dyn FnMut() -> T + 'static>> {
        let f = Box::new(compute_value) as _;
        DerivationPtr::new(f)
    }

    pub fn computed(compute_value: F) -> Self {
        Self::new(compute_value)
    }

    pub fn borrow(&self) -> Ref<T> {
        static_state::note_observed(Rc::clone(&self.ptr) as _);
        self.ptr.value.borrow()
    }

    pub fn borrow_untracked(&self) -> Ref<T> {
        self.ptr.value.borrow()
    }
}
