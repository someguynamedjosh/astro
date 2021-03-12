use num::ToPrimitive;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub xx: f32,
    pub yx: f32,
    pub ix: f32,
    pub xy: f32,
    pub yy: f32,
    pub iy: f32,
}

impl Transform {
    // Left to right, then top to bottom, like a 2 row 3 col matrix.
    pub const fn new(xx: f32, yx: f32, ix: f32, xy: f32, yy: f32, iy: f32) -> Self {
        Self {
            xx,
            yx,
            ix,
            xy,
            yy,
            iy,
        }
    }

    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub type Size = Vec2;
pub type Position = Vec2;

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

macro_rules! from_scalar {
    ($($T:ty),*) => {
        $(impl From<$T> for Vec2 {
            fn from(other: $T) -> Self {
                let value = other as f32;
                Self::new(value, value)
            }
        })*
    };
}

from_scalar!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

impl<T: ToPrimitive, U: ToPrimitive> From<(T, U)> for Vec2 {
    fn from(other: (T, U)) -> Self {
        Self::new(other.0.to_f32().unwrap(), other.1.to_f32().unwrap())
    }
}

macro_rules! op_impl {
    ($trait_name:ident, $fn_name:ident, $op_symbol:tt) => {
        impl<R: Into<Vec2>> $trait_name<R> for Vec2 {
            type Output = Vec2;
            fn $fn_name(self, rhs: R) -> Self::Output {
                let rhs = rhs.into();
                Self {
                    x: self.x $op_symbol rhs.x,
                    y: self.y $op_symbol rhs.y,
                }
            }
        }
    };
}

op_impl!(Add, add, +);
op_impl!(Sub, sub, -);
op_impl!(Mul, mul, *);
op_impl!(Div, div, /);
op_impl!(Rem, rem, %);

macro_rules! op_assign_impl {
    ($trait_name:ident, $fn_name:ident, $op_symbol:tt) => {
        impl<R: Into<Vec2>> $trait_name<R> for Vec2 {
            fn $fn_name(&mut self, rhs: R) {
                let rhs = rhs.into();
                self.x $op_symbol rhs.x;
                self.y $op_symbol rhs.y;
            }
        }
    };
}

op_assign_impl!(AddAssign, add_assign, +=);
op_assign_impl!(SubAssign, sub_assign, -=);
op_assign_impl!(MulAssign, mul_assign, *=);
op_assign_impl!(DivAssign, div_assign, /=);
op_assign_impl!(RemAssign, rem_assign, %=);
