#![feature(arc_new_cyclic)]

use crossbeam::atomic::AtomicCell;
use std::{
    cell::{Cell, Ref, RefCell},
    rc::{Rc, Weak},
    thread::{self, ThreadId},
};

// This might mistakenly be accessed from more than one thread. To guarantee that we correctly
// generate an error (and therefore prevent UB later on) we use guaranteed safe types.
static MAIN_THREAD: AtomicCell<Option<ThreadId>> = AtomicCell::new(None);
// Accessing these are safe as long as you first call assert_static_state_access() which checks that
// we are on MAIN_THREAD, which never changes after an initial call to init().
// https://stackoverflow.com/questions/37060330/safe-way-to-push-the-local-value-into-a-static-mut
// static mut is safe if you are only ever accessing it from a single thread and if it is impossible
// to hold more than one mutable reference at a time, check for reentrance!
static mut OBSERVING_STACK: Vec<Vec<Rc<dyn ObservableInternalFns>>> = Vec::new();

pub fn init() {
    if MAIN_THREAD.load().is_some() {
        panic!("Called init() a second time.");
    }
    MAIN_THREAD.store(Some(thread::current().id()));
}

pub fn is_initialized() -> bool {
    MAIN_THREAD.load().is_some()
}

/// Panics if init() has not been called or if called from a different thread than init() was called
/// on.
fn assert_static_state_access() {
    let this_thread = Some(thread::current().id());
    let mt = MAIN_THREAD.load();
    if mt != this_thread {
        if let Some(id) = mt {
            panic!(
                concat!(
                    "a function was just called from thread {:?} ",
                    "but observatory::init() was called from thread {:?}"
                ),
                thread::current().id(),
                id
            );
        } else {
            panic!(concat!(
                "Function called before initialization, ",
                "are you missing a call to observatory::init()?"
            ));
        }
    }
}

fn push_observing_stack() {
    assert_static_state_access();
    unsafe { OBSERVING_STACK.push(Vec::new()) }
}

fn note_observed(observable: Rc<dyn ObservableInternalFns>) {
    assert_static_state_access();
    if let Some(item) = unsafe { OBSERVING_STACK.last_mut() } {
        item.push(observable);
    } else {
        panic!(
            "Observable borrowed outside of derivation. Did you mean to use borrow_untracked()?"
        );
    }
}

fn pop_observing_stack() -> Vec<Rc<dyn ObservableInternalFns>> {
    assert_static_state_access();
    let top = unsafe { OBSERVING_STACK.pop() };
    if let Some(value) = top {
        value
    } else {
        panic!("(Internal error) pop() called more times than push()");
    }
}

/// Helper struct which stores observers that should be notified whenever an observable object
/// changes. Used by both Observable and Derivation.
#[derive(Default)]
struct ObserverList {
    observers: Cell<Vec<Weak<dyn DerivationInternalFns>>>,
}

impl ObserverList {
    fn broadcast_stale(&self) {
        let list = self.observers.take();
        for observer in &list {
            observer.upgrade().map(|v| v.send_stale());
        }
        self.observers.set(list);
    }

    fn broadcast_ready(&self, changed: bool) {
        let list = self.observers.take();
        for observer in &list {
            observer.upgrade().map(|v| v.send_ready(changed));
        }
        self.observers.set(list);
    }

    fn subscribe(&self, observer: Weak<dyn DerivationInternalFns>) {
        let mut list = self.observers.take();
        list.push(observer);
        self.observers.set(list);
    }

    fn unsubscribe(&self, observer: &Weak<dyn DerivationInternalFns>) {
        let mut list = self.observers.take();
        let index = list.iter().position(|item| Weak::ptr_eq(item, observer));
        let index = index.expect(
            "(Internal error) Tried to unsubscribe a derivation that was already unsubscribed.",
        );
        list.remove(index);
        self.observers.set(list);
    }
}

trait ObservableInternalFns {
    fn subscribe(&self, derivation: Weak<dyn DerivationInternalFns>);
    fn unsubscribe(&self, derivation: &Weak<dyn DerivationInternalFns>);
}

trait DerivationInternalFns {
    fn send_stale(&self);
    fn send_ready(&self, changed: bool);
    fn update(&self);
}

#[repr(C)]
struct DerivationBox<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    this_ptr: Weak<dyn DerivationInternalFns>,
    num_stale_notifications: Cell<usize>,
    observers: ObserverList,
    observing: Cell<Vec<Rc<dyn ObservableInternalFns>>>,
    /// True if fields we are observing have changed and we need to update once
    /// num_stale_notifications reaches zero.
    should_update: Cell<bool>,
    compute_value: RefCell<F>,
    value: RefCell<T>,
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> DerivationInternalFns
    for DerivationBox<T, F>
{
    /// Called when a value this derivation depends on becomes stale.
    fn send_stale(&self) {
        self.num_stale_notifications
            .set(self.num_stale_notifications.get() + 1);
        self.observers.broadcast_stale();
    }

    /// Called when a value this derivation depends on finishes updating. `changed` is false if the
    /// value has not changed.
    fn send_ready(&self, changed: bool) {
        let nsn = self.num_stale_notifications.get() - 1;
        self.num_stale_notifications.set(nsn);
        let should_update = self.should_update.get() || changed;
        self.should_update.set(should_update);
        if nsn == 0 && should_update {
            drop(self);
            self.update();
        }
    }

    fn update(&self) {
        assert!(self.should_update.get());
        self.should_update.set(false);

        push_observing_stack();
        let new_value = (self.compute_value.borrow_mut())();
        let now_observing = pop_observing_stack();
        let was_observing = self.observing.take();
        for observable in &was_observing {
            // If we are no longer observing something we used to...
            if !now_observing
                .iter()
                .any(|other| Rc::ptr_eq(observable, other))
            {
                observable.unsubscribe(&self.this_ptr)
            }
        }

        let changed = new_value != *self.value.borrow();
        if changed {
            self.value.replace(new_value);
        }

        self.observers.broadcast_ready(changed);
    }
}

impl<T: PartialEq, F: FnMut() -> T> ObservableInternalFns for DerivationBox<T, F> {
    fn subscribe(&self, derivation: Weak<dyn DerivationInternalFns>) {
        self.observers.subscribe(derivation);
    }

    fn unsubscribe(&self, derivation: &Weak<dyn DerivationInternalFns>) {
        self.observers.unsubscribe(derivation);
    }
}

#[derive(Clone)]
pub struct Derivation<T: PartialEq + 'static, F: FnMut() -> T + 'static> {
    ptr: Rc<DerivationBox<T, F>>,
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> Derivation<T, F> {
    fn new(mut compute_value: F) -> Self {
        push_observing_stack();
        let initial_value = compute_value();
        let observing = pop_observing_stack();
        let ptr = Rc::new_cyclic(|weak| DerivationBox {
            this_ptr: Weak::clone(weak) as _,
            num_stale_notifications: Cell::new(0),
            observers: Default::default(),
            observing: Cell::new(observing.clone()),
            should_update: Cell::new(false),
            compute_value: RefCell::new(compute_value),
            value: RefCell::new(initial_value),
        });
        for observable in &observing {
            observable.subscribe(Rc::downgrade(&ptr) as _);
        }
        Self { ptr }
    }

    pub fn computed(compute_value: F) -> Self {
        Self::new(compute_value)
    }

    pub fn borrow(&self) -> Ref<T> {
        note_observed(Rc::clone(&self.ptr) as _);
        self.ptr.value.borrow()
    }

    pub fn borrow_untracked(&self) -> Ref<T> {
        self.ptr.value.borrow()
    }
}

#[repr(C)]
struct ObservableBox<T: ?Sized> {
    observers: ObserverList,
    value: RefCell<T>,
}

impl<T: PartialEq> ObservableInternalFns for ObservableBox<T> {
    fn subscribe(&self, derivation: Weak<dyn DerivationInternalFns>) {
        self.observers.subscribe(derivation);
    }

    fn unsubscribe(&self, derivation: &Weak<dyn DerivationInternalFns>) {
        self.observers.unsubscribe(derivation);
    }
}

#[derive(Clone)]
pub struct Observable<T: ?Sized + PartialEq + 'static> {
    ptr: Rc<ObservableBox<T>>,
}

impl<T: PartialEq + 'static> Observable<T> {
    pub fn new(value: T) -> Self {
        let bx = ObservableBox {
            observers: Default::default(),
            value: RefCell::new(value),
        };
        let ptr = Rc::new(bx);
        Self { ptr }
    }

    pub fn borrow(&self) -> Ref<T> {
        note_observed(Rc::clone(&self.ptr) as _);
        self.ptr.value.borrow()
    }

    pub fn borrow_untracked(&self) -> Ref<T> {
        self.ptr.value.borrow()
    }

    pub fn set(&self, value: T) {
        if value == *self.ptr.value.borrow() {
            return;
        }
        self.ptr.value.replace(value);
        self.after_modified();
    }

    fn after_modified(&self) {
        self.ptr.observers.broadcast_stale();
        self.ptr.observers.broadcast_ready(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_if_needed() {
        if !is_initialized() {
            init();
        }
    }

    #[test]
    fn shared_ptr_behavior() {
        let value = Observable::new(123);
        assert_eq!(*value.borrow_untracked(), 123);
        let value2 = Observable::clone(&value);
        assert_eq!(*value2.borrow_untracked(), 123);
        assert_eq!(*value.borrow_untracked(), 123);
        value2.set(42);
        assert_eq!(*value.borrow_untracked(), 42);
        assert_eq!(*value2.borrow_untracked(), 42);
    }

    #[test]
    fn update_immediate_derivation() {
        init_if_needed();
        let value = Observable::new(123);
        let value2 = Observable::clone(&value);
        let derived = Derivation::new(move || *value.borrow() + 1);
        assert_eq!(*derived.borrow_untracked(), 124);
        value2.set(42);
        assert_eq!(*derived.borrow_untracked(), 43);
    }

    #[test]
    fn update_chained_derivation() {
        init_if_needed();
        let value = Observable::new(0);
        let value2 = Observable::clone(&value);
        let deriveda1 = Derivation::new(move || *value.borrow() + 1);
        let deriveda2 = Derivation::clone(&deriveda1);
        let derivedb = Derivation::new(move || *deriveda1.borrow() + 1);

        assert_eq!(*value2.borrow_untracked(), 0);
        assert_eq!(*deriveda2.borrow_untracked(), 1);
        assert_eq!(*derivedb.borrow_untracked(), 2);

        value2.set(10);

        assert_eq!(*value2.borrow_untracked(), 10);
        assert_eq!(*deriveda2.borrow_untracked(), 11);
        assert_eq!(*derivedb.borrow_untracked(), 12);
    }

    #[test]
    fn update_only_once() {
        init_if_needed();
        let base = Observable::new(0);

        let intermediates: Vec<_> = (1..9)
            .map(|index| {
                let base2 = Observable::clone(&base);
                Derivation::new(move || *base2.borrow() + index)
            })
            .collect();

        let num_updates = Rc::new(Cell::new(0));
        let num_updates2 = Rc::clone(&num_updates);
        let result = Derivation::new(move || {
            num_updates.set(num_updates.get() + 1);
            intermediates
                .iter()
                .map(|value| *value.borrow())
                .fold(0, std::ops::Add::add)
        });

        assert_eq!(num_updates2.get(), 1);
        base.set(1);
        assert_eq!(num_updates2.get(), 2);

        drop(result);
    }
}
