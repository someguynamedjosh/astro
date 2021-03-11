use crate::{DerivationPtr, ObservablePtr};

#[doc(hidden)]
pub trait PtrUtil {
    fn ptr_clone(&self) -> Self;
}

impl<T: ?Sized + PartialEq + 'static> PtrUtil for ObservablePtr<T> {
    fn ptr_clone(&self) -> Self {
        Self::clone(&self)
    }
}

impl<T: PartialEq + 'static, F: FnMut() -> T + 'static> PtrUtil for DerivationPtr<T, F> {
    fn ptr_clone(&self) -> Self {
        Self::clone(&self)
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __expr_result_name {
    ($plain:ident) => {
        $plain
    };
    ($fn_call:ident ( )) => {
        $fn_call
    };
    ($some:ident . 0) => { $some };
    ($some:ident . 1) => { $some };
    ($some:ident . 2) => { $some };
    ($some:ident . 3) => { $some };
    ($some:ident . 4) => { $some };
    ($some:ident . 5) => { $some };
    ($some:ident . 6) => { $some };
    ($some:ident . 7) => { $some };
    ($some:ident . 8) => { $some };
    ($some:ident . 9) => { $some };
    ($some:ident . 10) => { $some };
    ($some:ident . 11) => { $some };
    ($some:ident . 12) => { $some };
    ($some:ident . 13) => { $some };
    ($some:ident . 14) => { $some };
    ($some:ident . 15) => { $some };
    ($some:ident . 16) => { $some };
    ($some:ident . 17) => { $some };
    ($some:ident . 18) => { $some };
    ($some:ident . 19) => { $some };
    ($some:ident . 20) => { $some };
    ($some:ident . 21) => { $some };
    ($some:ident . 22) => { $some };
    ($some:ident . 23) => { $some };
    ($some:ident . 24) => { $some };
    ($some:ident . 25) => { $some };
    ($some:ident . 26) => { $some };
    ($some:ident . 27) => { $some };
    ($some:ident . 28) => { $some };
    ($some:ident . 29) => { $some };
    ($some:ident . 30) => { $some };
    ($some:ident . 31) => { $some };
    ($some:ident . $($rest:tt)*) => {
        __expr_result_name!($($rest)*)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __ptr_clone_line {
    ($name:ident : $($ex:tt)*) => {
        let $name = $crate::ptr_util::PtrUtil::ptr_clone(&$($ex)*);
    };
    ($($ex:tt)*) => {
        let $crate::__expr_result_name!($($ex)*) = $crate::ptr_util::PtrUtil::ptr_clone(&$($ex)*);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __ptr_clone {
    ($(($($ex:tt)*)),+) => {
        $($crate::__ptr_clone_line!($($ex)*);)+
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __ptr_clone_parse {
    ([$($args:tt)*], ($($wip:tt)*), , $($rest:tt)*) => {
        $crate::__ptr_clone_parse!([$($args)* ($($wip)*)], (), $($rest)*)
    };
    ([$($args:tt)*], ($($wip:tt)*), $next:tt $($rest:tt)*) => {
        $crate::__ptr_clone_parse!([$($args)*], ($($wip)*$next), $($rest)*)
    };
    ([$($args:tt)*], ($($wip:tt)*),) => {
        $crate::__ptr_clone!(($($wip)*) $(,$args)*)
    };
}

#[macro_export]
macro_rules! ptr_clone {
    () => {};
    ($($ex:tt)+) => {
        $crate::__ptr_clone_parse!([], (), $($ex)+)
    }
}
