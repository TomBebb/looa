#[macro_use]
extern crate nom;

use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::collections::HashMap;
use std::ops::Drop;
use std::any::Any;
use std::cmp::{PartialOrd, PartialEq, Ordering};
use std::mem;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Type {
    Nil,
    Boolean,
    Number,
    String,
    Function,
    Userdata,
    Thread,
    Table
}

pub trait ConvertValue: Sized {
    const TYPE: Type;
    fn into_value(self) -> Value {
        let data = InnerValueData {
            ty: Self::TYPE,
            val: self
        };
        Value {
            data: unsafe {
                mem::transmute(Rc::new(data))
            }
        }
    }
    unsafe fn from_value_raw(val: &Value) -> &Self {
        let data: &ValueData = &*val.data;
        let data: &InnerValueData<Self> = mem::transmute(data);
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
impl ConvertValue for () {
    const TYPE: Type = Type::Nil;
}
impl ConvertValue for bool {
    const TYPE: Type = Type::Boolean;
}
impl ConvertValue for f32 {
    const TYPE: Type = Type::Number;
}
impl ConvertValue for String {
    const TYPE: Type = Type::String;
}
impl ConvertValue for Box<Any> {
    const TYPE: Type = Type::Userdata;
}

struct ValueData {
    ty: Type
}

struct InnerValueData<T> {
    ty: Type,
    val: T
}

#[derive(Clone)]
pub struct Value {
    data: Rc<ValueData>
}
impl Value {
    pub fn nil() -> Value {
        Value::new(())
    }
    pub fn new<T>(val: T) -> Value where T: ConvertValue {
        ConvertValue::into_value(val)
    }
    pub fn type_of(&self) -> Type {
        self.data.ty
    }
    unsafe fn drop<T>(&mut self) {
        let mut other = Rc::new(ValueData {
            ty: Type::Nil
        });
        mem::swap(&mut self.data, &mut other);
        let _: Rc<InnerValueData<T>> = mem::transmute(other);
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
                    Type::Number => f32::from_value_raw(self) == f32::from_value_raw(other),
                    _ => unimplemented!()
                }
            }
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<Ordering> {
        let (ty, other_ty) = (self.type_of(), other.type_of());
        if ty != other_ty {
            None
        } else {
            unsafe {
                    match ty {
                        Type::Nil => Some(Ordering::Equal),
                        Type::Boolean => PartialOrd::partial_cmp(bool::from_value_raw(self), bool::from_value_raw(other)),
                        Type::Number => PartialOrd::partial_cmp(f32::from_value_raw(self), f32::from_value_raw(other)),
                        _ => unimplemented!()
                    }
            }
        }
    }
}
impl Drop for Value {
    fn drop(&mut self) {
        unsafe {
            match self.type_of() {
                Type::Nil => (),
                Type::Boolean => Value::drop::<bool>(self),
                Type::Number => Value::drop::<f32>(self),
                Type::String => Value::drop::<String>(self),
                _ => unimplemented!()
            }
        }
    }
}

fn main() {
    let val = Value::new(false);
    println!("Hello, world!");
}
