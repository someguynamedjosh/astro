use crate::{
    observable::ObservableInternalFns,
    ptr::{ThinPtr, WeakThinPtr},
    static_state,
};
use std::cell::{Cell, Ref, RefCell};

pub(crate) trait ObserverInternalFns {
    fn send_stale(&self);
    fn send_ready(&self, changed: bool);
    fn update(&self);
}

/// Helper struct which stores observers that should be notified whenever an observable object
/// changes. Used by both Observable and Derivation.
#[derive(Default)]
pub(crate) struct ObserverList {
    observers: Cell<Vec<WeakThinPtr<dyn ObserverInternalFns>>>,
}

impl ObserverList {
    pub fn broadcast_stale(&self) {
        let list = self.observers.take();
        for observer in &list {
            unsafe { observer.deref() }.send_stale();
        }
        self.observers.set(list);
    }

    pub fn broadcast_ready(&self, changed: bool) {
        let list = self.observers.take();
        for observer in &list {
            unsafe { observer.deref() }.send_ready(changed);
        }
        self.observers.set(list);
    }

    pub fn add(&self, observer: WeakThinPtr<dyn ObserverInternalFns>) {
        let mut list = self.observers.take();
        list.push(observer);
        self.observers.set(list);
    }

    pub fn remove(&self, observer: &WeakThinPtr<dyn ObserverInternalFns>) {
        let mut list = self.observers.take();
        let index = list
            .iter()
            .position(|item| WeakThinPtr::ptr_eq(item, observer));
        let index = index.expect(
            "(Internal error) Tried to unsubscribe an observer that was already unsubscribed.",
        );
        list.remove(index);
        self.observers.set(list);
    }
}

#[repr(C)]
struct DerivationData<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    this_ptr: WeakThinPtr<dyn ObserverInternalFns>,
    observers: ObserverList,
    observing: Cell<Vec<ThinPtr<dyn ObservableInternalFns>>>,
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
        self.num_stale_notifications
            .set(self.num_stale_notifications.get() + 1);
        self.observers.broadcast_stale();
    }

    /// Called when a value this observer depends on finishes updating. `changed` is false if the
    /// value has not changed.
    fn send_ready(&self, changed: bool) {
        let nsn = self.num_stale_notifications.get() - 1;
        self.num_stale_notifications.set(nsn);
        let should_update = self.should_update.get() || changed;
        self.should_update.set(should_update);
        if nsn == 0 && should_update {
            self.update();
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
            // If we are no longer observing something we used to...
            if !now_observing
                .iter()
                .any(|other| ThinPtr::ptr_eq(observable, other))
            {
                observable.remove_observer(&self.this_ptr)
            }
        }
        for observable in &now_observing {
            // If we are observing something we weren't observing before...
            if !was_observing
                .iter()
                .any(|other| ThinPtr::ptr_eq(observable, other))
            {
                observable.add_observer(WeakThinPtr::clone(&self.this_ptr));
            }
        }

        let changed = new_value != *self.value.borrow();
        if changed {
            self.value.replace(new_value);
        }

        self.observers.broadcast_ready(changed);
    }
}

impl<T: PartialEq, F: FnMut() -> T> ObservableInternalFns for DerivationData<T, F> {
    fn add_observer(&self, observer: WeakThinPtr<dyn ObserverInternalFns>) {
        self.observers.add(observer);
    }

    fn remove_observer(&self, observer: &WeakThinPtr<dyn ObserverInternalFns>) {
        self.observers.remove(observer);
    }
}

pub struct DerivationPtr<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    ptr: ThinPtr<DerivationData<T, F>>,
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> Clone for DerivationPtr<T, F> {
    fn clone(&self) -> Self {
        Self {
            ptr: ThinPtr::clone(&self.ptr),
        }
    }
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> DerivationPtr<T, F> {
    pub fn new(mut compute_value: F) -> Self {
        static_state::push_observing_stack();
        let initial_value = compute_value();
        let observing = static_state::pop_observing_stack();
        let ptr = ThinPtr::new_cyclic(|weak| DerivationData {
            this_ptr: WeakThinPtr::clone(weak) as _,
            num_stale_notifications: Cell::new(0),
            observers: Default::default(),
            observing: Cell::new(observing.clone()),
            should_update: Cell::new(false),
            compute_value: RefCell::new(compute_value),
            value: RefCell::new(initial_value),
        });
        for observable in &observing {
            observable.add_observer(ThinPtr::downgrade(&ptr) as _);
        }
        Self { ptr }
    }

    pub fn computed(compute_value: F) -> Self {
        Self::new(compute_value)
    }

    pub fn borrow(&self) -> Ref<T> {
        static_state::note_observed(ThinPtr::clone(&self.ptr) as _);
        self.ptr.value.borrow()
    }

    pub fn borrow_untracked(&self) -> Ref<T> {
        self.ptr.value.borrow()
    }
}
