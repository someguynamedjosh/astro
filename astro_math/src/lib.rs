use num::ToPrimitive;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

#[derive(Clone, Copy, Debug, PartialEq)]
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
        Self::new(1.0, 0.0, 0.0, 0.0, 1.0, 0.0)
    }

    pub const fn translate(offset: Vec2) -> Self {
        Self::new(1.0, 0.0, offset.x, 0.0, 1.0, offset.y)
    }

    pub fn translated(self, offset: Vec2) -> Self {
        self * Self::translate(offset)
    }

    pub const fn scale(amount: Size) -> Self {
        Self::new(amount.x, 0.0, 0.0, 0.0, amount.y, 0.0)
    }

    pub fn scaled(self, amount: Size) -> Self {
        self * Self::scale(amount)
    }
}

impl Mul for Transform {
    type Output = Transform;
    fn mul(self, rhs: Transform) -> Self::Output {
        Self {
            xx: self.xx * rhs.xx + self.xy * rhs.yx,
            yx: self.yx * rhs.xx + self.yy * rhs.yx,
            ix: self.ix * rhs.xx + self.iy * rhs.yx + rhs.ix,
            xy: self.xx * rhs.xy + self.xy * rhs.yy,
            yy: self.xy * rhs.yx + self.yy * rhs.yy,
            iy: self.ix * rhs.xy + self.iy * rhs.yy + rhs.iy,
        }
    }
}

impl MulAssign for Transform {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub type Size = Vec2;
pub type Point = Vec2;

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

impl Mul<Transform> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Transform) -> Self::Output {
        Vec2::new(
            self.x * rhs.xx + self.y * rhs.yx + rhs.ix,
            self.x * rhs.xy + self.y * rhs.yy + rhs.iy,
        )
    }
}

impl MulAssign<Transform> for Vec2 {
    fn mul_assign(&mut self, rhs: Transform) {
        *self = *self * rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::{Transform, Vec2};

    #[test]
    fn translate() {
        assert_eq!(
            Transform::translate(Vec2::new(3.0, 4.0)),
            Transform::new(1.0, 0.0, 3.0, 0.0, 1.0, 4.0),
        )
    }

    #[test]
    fn translate_identity() {
        assert_eq!(
            Transform::identity(),
            Transform::translate(Vec2::new(1.0, 1.0)).translated(Vec2::new(-1.0, -1.0))
        );
    }

    #[test]
    fn translate_point() {
        assert_eq!(
            Vec2::new(1.0, 2.0) * Transform::translate(Vec2::new(3.0, 4.0)),
            Vec2::new(4.0, 6.0)
        )
    }

    #[test]
    fn scale() {
        assert_eq!(
            Transform::scale(Vec2::new(3.0, 4.0)),
            Transform::new(3.0, 0.0, 0.0, 0.0, 4.0, 0.0),
        )
    }

    #[test]
    fn scale_identity() {
        assert_eq!(
            Transform::identity(),
            Transform::scale(Vec2::new(2.0, 4.0)).scaled(Vec2::new(0.5, 0.25))
        );
    }

    #[test]
    fn scale_point() {
        assert_eq!(
            Vec2::new(1.0, 2.0) * Transform::scale(Vec2::new(3.0, 4.0)),
            Vec2::new(3.0, 8.0)
        )
    }
}
