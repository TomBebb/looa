#[macro_use]
extern crate nom;

use std::any::Any;
use std::cmp::{Ordering, Ord, PartialEq, PartialOrd, Eq};

use std::hash::{Hash, Hasher};
use std::collections::BTreeMap;
use std::fmt;
use std::mem;
use std::ops::{Add, Sub, Mul, Div, Drop};
use std::rc::Rc;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, PartialOrd, Ord)]
pub enum Type {
    /// represents the absence of a value
    Nil,
    /// has two values, `false` and `true`
    Boolean,
    /// represents both integer numbers and real (floating-point) numbers
    Number,
    /// represents immutable sequences of bytes
    String,
    /// callable Rust or Lua function
    Function,
    Userdata,
    /// represents independent threads of execution and used to implement coroutines
    Thread,
    /// implements associative arrays, that is, arrays that can have as indices not only numbers, but any Lua value except nil and NaN
    Table,
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            Type::Nil => "nil",
            Type::Boolean => "boolean",
            Type::Number => "number",
            Type::String => "string",
            Type::Function => "function",
            Type::Userdata => "userdata",
            Type::Thread => "thread",
            Type::Table => "table"
        })
    }
}

pub trait ConvertValue: Sized {
    const TYPE: Type;
    fn into_value(self) -> Value {
        let data = InnerValueData {
            _ty: Self::TYPE,
            val: self,
        };
        let wrapped = Rc::new(data);
        Value {
            data: unsafe { mem::transmute(wrapped) },
        }
    }
    unsafe fn from_value_raw(val: &Value) -> &Self {
        let data: &ValueData = &*val.data;
        let data: &InnerValueData<Self> =
            &*(data as *const ValueData as *const InnerValueData<Self>);
        &data.val
    }
    fn from_value(val: &Value) -> Option<&Self> {
        unsafe {
            if val.type_of() == Self::TYPE {
                Some(Self::from_value_raw(val))
            } else {
                None
            }
        }
    }
}

pub type LuaNil = ();
pub type LuaBool = bool;
pub type LuaNumber = f32;
pub type LuaString = Box<u8>;
pub type LuaUserdata = Box<Any>;
pub type LuaTable = BTreeMap<Value, Value>;

impl ConvertValue for LuaNil {
    const TYPE: Type = Type::Nil;
}
impl ConvertValue for LuaBool {
    const TYPE: Type = Type::Boolean;
}
impl ConvertValue for LuaNumber {
    const TYPE: Type = Type::Number;
}
impl ConvertValue for LuaString {
    const TYPE: Type = Type::String;
}
impl ConvertValue for LuaUserdata {
    const TYPE: Type = Type::Userdata;
}
impl ConvertValue for LuaTable {
    const TYPE: Type = Type::Table;
}

#[repr(packed)]
struct ValueData {
    ty: Type,
}

#[repr(packed)]
struct InnerValueData<T> {
    _ty: Type,
    val: T,
}

#[derive(Clone)]
pub struct Value {
    data: Rc<ValueData>,
}
impl Value {
    pub fn nil() -> Value {
        Value::new(())
    }
    pub fn new<T>(val: T) -> Value
    where
        T: ConvertValue,
    {
        ConvertValue::into_value(val)
    }
    pub fn type_of(&self) -> Type {
        self.data.ty
    }
    pub fn is_index(&self) -> bool {
        unsafe {
            match self.type_of() {
                Type::Nil => false,
                Type::Number => LuaNumber::from_value_raw(self) == &::std::f32::NAN,
                _ => true
            }
        }
    }
    pub fn to_bool(&self) -> bool {
        unsafe {
            match self.type_of() {
                Type::Nil => false,
                Type::Boolean => *LuaBool::from_value_raw(self),
                _ => true
            }
        }
    }
    pub fn get_index(&self, index: &Value) -> Value {
        if let Some(table) = LuaTable::from_value(self) {
            let val: Option<Value> = table.get(index).cloned();
            val.unwrap_or_else(|| Value::nil())
        } else {
            Value::nil()
        }
    }

    unsafe fn drop<T>(&mut self) {
        let mut other = Rc::new(ValueData { ty: Type::Nil });
        mem::swap(&mut self.data, &mut other);
        let _: Rc<InnerValueData<T>> = mem::transmute(other);
    }
    unsafe fn num_binop<F>(a: &Value, b: &Value, op_str: &str, op: F) -> Value where F:Fn(LuaNumber, LuaNumber) -> LuaNumber {
        let (a_ty, b_ty) = (a.type_of(), b.type_of());
        if a_ty != b_ty {
            panic!("cannot resolve {} {} {}; cannot add {} and {}; must be same type", a, op_str, b, a_ty, b_ty);
        }
        if a_ty != Type::Number {
            panic!("cannot add non-number values {}", a_ty);
        }
        LuaNumber::into_value(op(*LuaNumber::from_value_raw(a), *LuaNumber::from_value_raw(b)))
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match self.type_of() {
                Type::Nil => f.write_str("nil"),
                Type::Boolean => fmt::Display::fmt(bool::from_value_raw(self), f),
                Type::Number => fmt::Display::fmt(LuaNumber::from_value_raw(self), f),
                _ => unimplemented!(),
            }
        }
    }
}
impl Add for Value {
    type Output = Value;
    fn add(self: Self, other: Self) -> Self::Output {
        &self + &other
    }
}
impl<'a> Add for &'a Value {
    type Output = Value;
    fn add(self: Self, other: Self) -> Self::Output {
        unsafe {
            Value::num_binop(self, other, "+", LuaNumber::add)
        }
    }
}
impl Sub for Value {
    type Output = Value;
    fn sub(self: Self, other: Self) -> Self::Output {
        &self - &other
    }
}
impl<'a> Sub for &'a Value {
    type Output = Value;
    fn sub(self: Self, other: Self) -> Self::Output {
        unsafe {
            Value::num_binop(self, other, "-", LuaNumber::sub)
        }
    }
}
impl Mul for Value {
    type Output = Value;
    fn mul(self: Self, other: Self) -> Self::Output {
        &self * &other
    }
}
impl<'a> Mul for &'a Value {
    type Output = Value;
    fn mul(self: Self, other: Self) -> Self::Output {
        unsafe {
            Value::num_binop(self, other, "*", LuaNumber::mul)
        }
    }
}
impl Div for Value {
    type Output = Value;
    fn div(self: Self, other: Self) -> Self::Output {
        &self / &other
    }
}
impl<'a> Div for &'a Value {
    type Output = Value;
    fn div(self: Self, other: Self) -> Self::Output {
        unsafe {
            Value::num_binop(self, other, "/", LuaNumber::add)
        }
    }
}
impl Eq for Value {
}
impl Hash for Value {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.data.ty.hash(state);
        unsafe {
            match self.type_of() {
                Type::Nil => ().hash(state),
                Type::Boolean => bool::from_value_raw(self).hash(state),
                Type::Number => mem::transmute::<_,&i32>(LuaNumber::from_value_raw(self)).hash(state),
                Type::Table => LuaTable::from_value_raw(self).hash(state),
                _ => unimplemented!()
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        let (ty, other_ty) = (self.type_of(), other.type_of());
        if ty != other_ty {
            false
        } else {
            unsafe {
                match ty {
                    Type::Nil => true,
                    Type::Boolean => bool::from_value_raw(self) == bool::from_value_raw(other),
                    Type::Number => LuaNumber::from_value_raw(self) == LuaNumber::from_value_raw(other),
                    _ => unimplemented!(),
                }
            }
        }
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Value) -> Ordering {
        let (ty, other_ty) = (self.type_of(), other.type_of());
        if ty != other_ty {
            Ord::cmp(&ty, &other_ty)
        } else {
            unsafe {
                match ty {
                    Type::Nil => Ordering::Equal,
                    Type::Boolean => Ord::cmp(
                        bool::from_value_raw(self),
                        bool::from_value_raw(other),
                    ),
                    Type::Number => PartialOrd::partial_cmp(
                        LuaNumber::from_value_raw(self),
                        LuaNumber::from_value_raw(other),
                    ).expect("Poop"),
                    _ => unimplemented!(),
                }
            }
        }
    }
}
impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}
impl Drop for Value {
    fn drop(&mut self) {
        unsafe {
            match self.type_of() {
                Type::Nil => (),
                Type::Boolean => Value::drop::<bool>(self),
                Type::Number => Value::drop::<LuaNumber>(self),
                Type::String => Value::drop::<LuaString>(self),
                _ => unimplemented!(),
            }
        }
    }
}

fn main() {
    let a = 12f32.into_value();
    let b = 13f32.into_value();
    println!("{}", a * b);
}
