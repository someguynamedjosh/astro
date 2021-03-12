use num::ToPrimitive;
use std::{
    collections::HashMap,
    convert::TryInto,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign},
};

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Self::from_packed(0x000000_FF);
    pub const WHITE: Color = Self::from_packed(0xFFFFFF_FF);

    pub const fn from_packed(packed: u32) -> Self {
        Self {
            r: (packed >> 24) as _,
            g: ((packed >> 16) & 0xFF) as _,
            b: ((packed >> 8) & 0xFF) as _,
            a: ((packed >> 0) & 0xFF) as _,
        }
    }
}

#[derive(Clone, Debug)]
pub enum FillMode {
    Solid(Color),
}

#[derive(Clone, Debug)]
pub enum RenderCommand {
    Clear(FillMode),
    DrawRect {
        transform: Transform,
        top_left: Position,
        size: Size,
        fill: FillMode,
    },
}

#[derive(Default)]
struct Layer {
    command_buffer: Vec<RenderCommand>,
}

struct LayerGroup {
    layers: HashMap<i8, Layer>,
    subgroups: HashMap<i8, Vec<LayerGroup>>,
}

impl LayerGroup {
    fn new() -> Self {
        Self {
            layers: HashMap::new(),
            subgroups: HashMap::new(),
        }
    }

    fn borrow_layer_mut(&mut self, height: i8) -> &mut Layer {
        // We call this twice because of mutable borrow rules, hopefully it is easily
        // optimized away.
        if self.layers.get_mut(&height).is_some() {
            self.layers.get_mut(&height).unwrap()
        } else {
            self.layers.insert(height, Default::default());
            self.layers.get_mut(&height).unwrap()
        }
    }

    fn add_subgroup(&mut self, height: i8, subgroup: LayerGroup) {
        if let Some(list) = self.subgroups.get_mut(&height) {
            list.push(subgroup);
        } else {
            self.subgroups.insert(height, vec![subgroup]);
        }
    }
}

#[derive(Clone)]
struct RenderContextState {
    transform: Transform,
    fill_mode: FillMode,
    layer: i8,
}

impl RenderContextState {
    fn new() -> Self {
        Self {
            transform: Transform::identity(),
            fill_mode: FillMode::Solid(Color::WHITE),
            layer: 0,
        }
    }
}

pub struct RenderContext {
    layer_group_stack: Vec<(i8, LayerGroup)>,
    state_stack: Vec<RenderContextState>,
    state: RenderContextState,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            layer_group_stack: vec![(0, LayerGroup::new())],
            state_stack: Vec::new(),
            state: RenderContextState::new(),
        }
    }

    pub fn get_state_stack_size(&self) -> usize {
        self.state_stack.len()
    }

    pub fn push_state(&mut self) {
        self.state_stack.push(self.state.clone());
    }

    pub fn pop_state(&mut self) {
        debug_assert!(self.state_stack.len() > 0);
        self.state = self.state_stack.pop().unwrap();
    }

    pub fn set_transform(&mut self, new: Transform) {
        self.state.transform = new;
    }

    pub fn set_fill_mode(&mut self, new: FillMode) {
        self.state.fill_mode = new;
    }

    pub fn fill_solid_color(&mut self, color: Color) {
        self.set_fill_mode(FillMode::Solid(color));
    }

    pub fn get_layer_group_stack_size(&self) -> usize {
        self.layer_group_stack.len()
    }

    pub fn set_layer(&mut self, height_index: i8) {
        self.state.layer = height_index;
    }

    pub fn begin_layer_group(&mut self, height: i8) {
        self.layer_group_stack.push((height, LayerGroup::new()));
        self.push_state();
        self.set_layer(0);
    }

    fn top_layer_group(&mut self) -> &mut LayerGroup {
        &mut self.layer_group_stack.last_mut().unwrap().1
    }

    pub fn end_layer_group(&mut self) {
        debug_assert!(self.layer_group_stack.len() > 1);
        let (height, group) = self.layer_group_stack.pop().unwrap();
        self.top_layer_group().add_subgroup(height, group);
        self.pop_state();
    }

    pub fn do_command(&mut self, command: RenderCommand) {
        let layer = self.state.layer;
        self.top_layer_group()
            .borrow_layer_mut(layer)
            .command_buffer
            .push(command);
    }

    pub fn clear(&mut self) {
        let command = RenderCommand::Clear(self.state.fill_mode.clone());
        self.do_command(command);
    }

    pub fn draw_rect(&mut self, top_left: Position, size: Size) {
        let command = RenderCommand::DrawRect {
            transform: self.state.transform.clone(),
            top_left,
            size,
            fill: self.state.fill_mode.clone(),
        };
        self.do_command(command);
    }
}

pub trait GuiConfig {
    type Renderer;
}

#[derive(Clone, Copy, Debug)]
pub struct Mat2 {
    pub xx: f32,
    pub yx: f32,
    pub xy: f32,
    pub yy: f32,
}

pub type Transform = Mat2;

impl Mat2 {
    // Left to right, then top to bottom.
    pub const fn new(xx: f32, yx: f32, xy: f32, yy: f32) -> Self {
        Self { xx, yx, xy, yy }
    }

    pub const fn identity() -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0)
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

#[derive(Clone, Copy)]
pub struct SizeConstraint {
    pub min: Size,
    pub max: Size,
}

pub trait RenderWidget<C: GuiConfig> {
    fn layout(&mut self, constraint: SizeConstraint) -> Size;
}

pub enum Alignment {
    Start,
    Middle,
    End,
}

pub use Alignment::End as Right;
pub use Alignment::End as Bottom;
pub use Alignment::Middle;
pub use Alignment::Middle as Center;
pub use Alignment::Start as Left;
pub use Alignment::Start as Top;

pub struct AlignBox<W> {
    pub horizontal: Alignment,
    pub vertical: Alignment,
    child_pos: Position,
    child: W,
}

impl<C: GuiConfig, W: RenderWidget<C>> RenderWidget<C> for AlignBox<W> {
    fn layout(&mut self, constraint: SizeConstraint) -> Size {
        let child_size = self.child.layout(SizeConstraint {
            min: 0.into(),
            max: constraint.max,
        });
        self.child_pos = (constraint.max - child_size) / 2;
        constraint.max
    }
}
