#![cfg(test)]

use crate::*;
use std::{cell::Cell, rc::Rc};

fn init_if_needed() {
    if !is_initialized() {
        init();
    }
}

#[test]
fn shared_ptr_behavior() {
    let value = observable(123);
    assert_eq!(*value.borrow_untracked(), 123);
    let value2 = ObservablePtr::clone(&value);
    assert_eq!(*value2.borrow_untracked(), 123);
    assert_eq!(*value.borrow_untracked(), 123);
    value2.set(42);
    assert_eq!(*value.borrow_untracked(), 42);
    assert_eq!(*value2.borrow_untracked(), 42);
}

#[test]
fn update_immediate_derivation() {
    init_if_needed();
    let value = observable(123);
    let derived = derivation_with_ptrs!(value; *value.borrow() + 1);
    assert_eq!(*derived.borrow_untracked(), 124);
    value.set(42);
    assert_eq!(*derived.borrow_untracked(), 43);
}

#[test]
fn update_chained_derivation() {
    init_if_needed();
    let value = observable(0);
    let deriveda = derivation_with_ptrs!(value; *value.borrow() + 1);
    let derivedb = derivation_with_ptrs!(deriveda; *deriveda.borrow() + 1);

    assert_eq!(*value.borrow_untracked(), 0);
    assert_eq!(*deriveda.borrow_untracked(), 1);
    assert_eq!(*derivedb.borrow_untracked(), 2);

    value.set(10);

    assert_eq!(*value.borrow_untracked(), 10);
    assert_eq!(*deriveda.borrow_untracked(), 11);
    assert_eq!(*derivedb.borrow_untracked(), 12);
}

#[test]
fn subscribe_then_drop() {
    init_if_needed();
    let value = observable(0);
    let deriveda = derivation_with_ptrs!(value; *value.borrow() + 1);
    let derivedb = derivation_with_ptrs!(deriveda; *deriveda.borrow() + 1);
    drop(derivedb);
    value.set(10);
    assert_eq!(*deriveda.borrow_untracked(), 11);
}

#[test]
fn update_only_once() {
    init_if_needed();
    let base = observable(0);

    let intermediates: Vec<_> = (1..9)
        .map(|index| derivation_with_ptrs!(base; *base.borrow() + index))
        .collect();

    let num_updates = Rc::new(Cell::new(0));
    let num_updates2 = Rc::clone(&num_updates);
    let result = DerivationPtr::new(move || {
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

#[test]
fn conditionally_observe_second_observable() {
    init_if_needed();
    let condition = observable(false);
    let second = observable(10);

    let num_updates = Rc::new(Cell::new(0));
    let num_updates2 = Rc::clone(&num_updates);
    let result = derivation_with_ptrs!(condition, second; {
        num_updates.set(num_updates.get() + 1);
        if *condition.borrow() {
            *second.borrow()
        } else {
            0
        }
    });

    assert_eq!(num_updates2.get(), 1);
    assert_eq!(*result.borrow_untracked(), 0);
    second.set(15);
    assert_eq!(num_updates2.get(), 1);
    assert_eq!(*result.borrow_untracked(), 0);
    condition.set(true);
    assert_eq!(num_updates2.get(), 2);
    assert_eq!(*result.borrow_untracked(), 15);
    second.set(5);
    assert_eq!(num_updates2.get(), 3);
    assert_eq!(*result.borrow_untracked(), 5);
}

#[test]
fn fork_and_join() {
    init_if_needed();
    let value = observable(123);

    let left = {
        ptr_clone!(value);
        DerivationPtr::new(move || *value.borrow())
    };
    let right = {
        ptr_clone!(value);
        DerivationPtr::new(move || *value.borrow())
    };
    let joined = {
        ptr_clone!(left, right);
        DerivationPtr::new(move || *left.borrow() + *right.borrow())
    };
    let num_updates = Rc::new(Cell::new(0));
    let num_updates2 = Rc::clone(&num_updates);
    let after = {
        ptr_clone!(joined);
        DerivationPtr::new(move || {
            let old = num_updates.get();
            num_updates.set(old + 1);
            *joined.borrow()
        })
    };
    assert_eq!(num_updates2.get(), 1);
    assert_eq!(*after.borrow_untracked(), 123 * 2);
    value.set(42);
    assert_eq!(num_updates2.get(), 2);
    assert_eq!(*after.borrow_untracked(), 42 * 2);
}

#[test]
fn update_through_mut_ref() {
    init_if_needed();
    let value = observable(123);
    let value2 = ObservablePtr::clone(&value);
    let derived = DerivationPtr::new(move || *value.borrow() + 1);
    assert_eq!(*derived.borrow_untracked(), 124);
    *value2.borrow_mut() = 42;
    assert_eq!(*derived.borrow_untracked(), 43);
}

#[test]
fn ptr_clone_macro() {
    let value = observable(123);
    struct Holder {
        value: ObservablePtr<i32>,
    }
    let holder = Holder { value };
    let derived = {
        ptr_clone!(holder.value);
        DerivationPtr::new(move || *value.borrow())
    };
    assert_eq!(*derived.borrow_untracked(), 123);
    holder.value.set(42);
    assert_eq!(*derived.borrow_untracked(), 42);
}
