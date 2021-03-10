#![feature(arc_new_cyclic)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(test)]

//! Provides MobX style observables. Example:
//! ```rust
//! use observatory as o;
//! o::init();
//! let first_name = o::observable("William");
//! let last_name = o::observable("Riker");
//! let nickname = o::observable::<Option<&'static str>>(None);
//! // A derivation is run the first time it is created, and the guts of the Derivation type will
//! // detect that the function borrows nickname, first_name, and last_name during that time.
//! # let output_data = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
//! # let output_data2 = Clone::clone(&output_data);
//! let display_name = o::derivation_with_ptrs!(
//!     first_name, last_name, nickname;
//!     if let Some(name) = *nickname.borrow() {
//!         format!("{}", name)
//!     } else {
//!         format!("{} {}", *first_name.borrow(), *last_name.borrow())
//!     }
//! );
//! // Prints "William Riker"
//! let logger = o::derivation_with_ptrs!(
//!     display_name;
//! #   { output_data.borrow_mut().push(display_name.borrow().clone());
//!     println!("{}", *display_name.borrow())
//! #   }
//! );
//! // Prints "Will of Yam Riker"
//! first_name.set("Will of Yam");
//! // Prints "Number One"
//! // After executing this function the library will detect that `display_name` didn't need to
//! // borrow first_name or last_name to update its value.
//! nickname.set(Some("Number One"));
//! // Causes no updates, display_name has automatically unsubscribed from updates to last_name.
//! last_name.set("Something else");
//! # assert_eq!(*output_data2.borrow(), vec![format!("William Riker"), format!("Will of Yam Riker"), format!("Number One")]);
//! ```
//! ## Observables
//! The function `observable` returns a value of type `ObservablePtr<T>`.
//! These represent singular pieces of data which can be modified through `ObservablePtr::set` or
//! `ObservablePtr::borrow_mut`. The value itself works like the standard library's `Rc` shared
//! pointer, in that it can be cloned and shared around, as long as it is not sent between threads:
//! ```rust
//! use observatory as o;
//! o::init();
//! let data = o::observable(123);
//! let data2 = Clone::clone(&data);
//! data.set(42);
//! assert_eq!(*data2.borrow_untracked(), 42);
//! ```
//! Notice the use of `borrow_untracked` in the previous example. The normal `ObservablePtr::borrow`
//! is intended to be called from within the body of a derivation and as such will panic outside of
//! that environment. 
//! ## Derivations
//! Derivations themselves are a kind of observable, as their result can be 
//! observed. However, they do not have `set` or `borrow_mut` functions. Instead, a single function
//! is specified which computes the value the derivation should have. This function is then 
//! automatically re-run whenever any of the observables it had borrowed have changed. Here is an
//! example of a derivation:
//! ```rust
//! use observatory as o;
//! o::init();
//! let number /* ObservablePtr<i32> */ = o::observable(3);
//! let number2 = Clone::clone(&number);
//! let squared /* DerivationPtr<i32, [closure type]> */ = o::derivation(move || {
//!     let value = *number2.borrow();
//!     value * value
//! });
//! assert_eq!(*squared.borrow_untracked(), 9);
//! number.set(4);
//! assert_eq!(*squared.borrow_untracked(), 16);
//! ```
//! Cloning all the pointers you need to access from within the closure can be rather tedious, which
//! is where the `derivation_with_ptrs` macro comes in:
//! ```rust
//! use observatory as o;
//! o::init();
//! let number = o::observable(3);
//! let squared = o::derivation_with_ptrs!(
//!     number, extra_example: number; {
//!     let value = *number.borrow();
//!     assert_eq!(*number.borrow(), *extra_example.borrow());
//!     value * value
//! });
//! assert_eq!(*squared.borrow_untracked(), 9);
//! number.set(4);
//! assert_eq!(*squared.borrow_untracked(), 16);
//! ```
//! The `DerivationPtr` struct has two template parameters, one for the return type and another for
//! the type of the function. Because of this, it is not directly possible to store these pointers
//! in a struct or in a vector. To solve this, the function type can be made to be 
//! `Box<dyn FnMut() -> T>`, but this would introduce a lot of boilerplate. This is handled for you
//! if you use `derivation_dyn` or `o::derivation_with_pointers_dyn` and the type 
//! alias `DerivationDynPtr<T>`:
//! ```rust
//! use observatory as o;
//! o::init();
//! let number = o::observable(3);
//! let squared /* DerivationDynPtr<i32> */ = o::derivation_with_ptrs_dyn!(
//!     number, extra_example: number; {
//!     let value = *number.borrow();
//!     assert_eq!(*number.borrow(), *extra_example.borrow());
//!     value * value
//! });
//! assert_eq!(*squared.borrow_untracked(), 9);
//! number.set(4);
//! assert_eq!(*squared.borrow_untracked(), 16);
//! ```

mod bench;
mod observable;
mod observer;
#[doc(hidden)]
pub mod ptr_util;
mod static_state;
mod tests;

pub use observable::ObservablePtr;
pub use observer::DerivationPtr;
pub use static_state::{init, is_initialized};

pub type DerivationDynPtr<T> = DerivationPtr<T, Box<dyn FnMut() -> T + 'static>>;

pub fn observable<T: PartialEq + 'static>(value: T) -> ObservablePtr<T> {
    ObservablePtr::new(value)
}

pub fn derivation<T: PartialEq + 'static, F: FnMut() -> T + 'static>(
    compute_value: F,
) -> DerivationPtr<T, F> {
    DerivationPtr::new(compute_value)
}

pub fn derivation_dyn<T: PartialEq + 'static, F: FnMut() -> T + 'static>(
    compute_value: F,
) -> DerivationDynPtr<T> {
    DerivationPtr::new_dyn(compute_value)
}

#[macro_export]
#[doc(hidden)]
macro_rules! __derivation_with_ptrs_parse {
    ($constructor:ident ($($args:tt)*); $($remaining:tt)*) => {
        {
            $crate::ptr_clone!($($args)*);
            $crate::$constructor(move || $($remaining)*)
        }
    };
    ($constructor:ident ($($args:tt)*) $next:tt $($remaining:tt)*) => {
        $crate::__derivation_with_ptrs_parse!($constructor ($($args)*$next) $($remaining)*)
    };
}

#[macro_export]
macro_rules! derivation_with_ptrs {
    ($($args:tt)*) => {
        $crate::__derivation_with_ptrs_parse!(derivation () $($args)*)
    };
}

#[macro_export]
macro_rules! derivation_with_ptrs_dyn {
    ($($args:tt)*) => {
        $crate::__derivation_with_ptrs_parse!(derivation_dyn () $($args)*)
    };
}
