#![feature(arc_new_cyclic)]
#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(test)]

mod bench;
mod observable;
mod observer;
mod ptr_util;
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
macro_rules! __derivation_with_ptrs_parse {
    ($constructor:ident ($($args:tt)*); $($remaining:tt)*) => {
        {
            ptr_clone!($($args)*);
            $constructor(move || $($remaining)*)
        }
    };
    ($constructor:ident ($($args:tt)*) $next:tt $($remaining:tt)*) => {
        __derivation_with_ptrs_parse!($constructor ($($args)*$next) $($remaining)*)
    };
}

#[macro_export]
macro_rules! derivation_with_ptrs {
    ($($args:tt)*) => {
        __derivation_with_ptrs_parse!(derivation () $($args)*)
    };
}

#[macro_export]
macro_rules! derivation_with_ptrs_dyn {
    ($($args:tt)*) => {
        __derivation_with_ptrs_parse!(derivation_dyn () $($args)*)
    };
}
