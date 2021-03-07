#![feature(arc_new_cyclic)]
#![feature(unsize)]
#![feature(coerce_unsized)]

mod observable;
mod observer;
mod ptr;
mod static_state;

pub use observable::ObservablePtr;
pub use observer::DerivationPtr;
pub use static_state::{init, is_initialized};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    fn init_if_needed() {
        if !is_initialized() {
            init();
        }
    }

    #[test]
    fn shared_ptr_behavior() {
        let value = ObservablePtr::new(123);
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
        let value = ObservablePtr::new(123);
        let value2 = ObservablePtr::clone(&value);
        let derived = DerivationPtr::new(move || *value.borrow() + 1);
        assert_eq!(*derived.borrow_untracked(), 124);
        value2.set(42);
        assert_eq!(*derived.borrow_untracked(), 43);
    }

    #[test]
    fn update_chained_derivation() {
        init_if_needed();
        let value = ObservablePtr::new(0);
        let value2 = ObservablePtr::clone(&value);
        let deriveda1 = DerivationPtr::new(move || *value.borrow() + 1);
        let deriveda2 = DerivationPtr::clone(&deriveda1);
        let derivedb = DerivationPtr::new(move || *deriveda1.borrow() + 1);

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
        let base = ObservablePtr::new(0);

        let intermediates: Vec<_> = (1..9)
            .map(|index| {
                let base2 = ObservablePtr::clone(&base);
                DerivationPtr::new(move || *base2.borrow() + index)
            })
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
        let condition = ObservablePtr::new(false);
        let condition2 = ObservablePtr::clone(&condition);
        let second = ObservablePtr::new(10);
        let second2 = ObservablePtr::clone(&second);

        let num_updates = Rc::new(Cell::new(0));
        let num_updates2 = Rc::clone(&num_updates);
        let result = DerivationPtr::new(move || {
            num_updates.set(num_updates.get() + 1);
            if *condition.borrow() {
                *second.borrow()
            } else {
                0
            }
        });

        assert_eq!(num_updates2.get(), 1);
        assert_eq!(*result.borrow_untracked(), 0);
        second2.set(15);
        assert_eq!(num_updates2.get(), 1);
        assert_eq!(*result.borrow_untracked(), 0);
        condition2.set(true);
        assert_eq!(num_updates2.get(), 2);
        assert_eq!(*result.borrow_untracked(), 15);
        second2.set(5);
        assert_eq!(num_updates2.get(), 3);
        assert_eq!(*result.borrow_untracked(), 5);
    }

    #[test]
    fn update_through_mut_ref() {
        init_if_needed();
        let value = ObservablePtr::new(123);
        let value2 = ObservablePtr::clone(&value);
        let derived = DerivationPtr::new(move || *value.borrow() + 1);
        assert_eq!(*derived.borrow_untracked(), 124);
        *value2.borrow_mut() = 42;
        assert_eq!(*derived.borrow_untracked(), 43);
    }
}
