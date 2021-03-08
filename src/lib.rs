#![feature(arc_new_cyclic)]
#![feature(unsize)]
#![feature(coerce_unsized)]

mod observable;
mod observer;
mod ptr;
mod static_state;
mod tests;

pub use observable::ObservablePtr;
pub use observer::DerivationPtr;
pub use static_state::{init, is_initialized};
