use crate::{observable::ObservableInternalFns, ptr::ThinPtr};
use crossbeam::atomic::AtomicCell;
use std::thread::{self, ThreadId};

// This might mistakenly be accessed from more than one thread. To guarantee that we correctly
// generate an error (and therefore prevent UB later on) we use guaranteed safe types.
static MAIN_THREAD: AtomicCell<Option<ThreadId>> = AtomicCell::new(None);
// Accessing these are safe as long as you first call assert_static_state_access() which checks that
// we are on MAIN_THREAD, which never changes after an initial call to init().
// https://stackoverflow.com/questions/37060330/safe-way-to-push-the-local-value-into-a-static-mut
// static mut is safe if you are only ever accessing it from a single thread and if it is impossible
// to hold more than one mutable reference at a time, check for reentrance!
static mut OBSERVING_STACK: Vec<Vec<ThinPtr<dyn ObservableInternalFns>>> = Vec::new();

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

pub(crate) fn push_observing_stack() {
    assert_static_state_access();
    unsafe { OBSERVING_STACK.push(Vec::new()) }
}

pub(crate) fn note_observed(observable: ThinPtr<dyn ObservableInternalFns>) {
    assert_static_state_access();
    if let Some(item) = unsafe { OBSERVING_STACK.last_mut() } {
        item.push(observable);
    } else {
        panic!(
            "Observable borrowed outside of derivation. Did you mean to use borrow_untracked()?"
        );
    }
}

pub(crate) fn pop_observing_stack() -> Vec<ThinPtr<dyn ObservableInternalFns>> {
    assert_static_state_access();
    let top = unsafe { OBSERVING_STACK.pop() };
    if let Some(value) = top {
        value
    } else {
        panic!("(Internal error) pop() called more times than push()");
    }
}
