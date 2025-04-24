use crate::blackboard::EntryRef;

use super::expr::ExprResult;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string value.
    String(String),
    /// A float value.
    Float(f64),
    /// An integer value.
    Integer(i64),
    /// A boolean value.
    Boolean(bool),
    /// An empty value.
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// A string value.
    String,
    /// A float value.
    Float,
    /// An integer value.
    Integer,
    /// A boolean value.
    Boolean,
    /// An empty value.
    Empty,
}

#[derive(Debug)]
pub enum ValueOrAny {
    /// Expression value
    Value(Value),
    /// Value from the Blackboard
    Any(EntryRef),
}

#[derive(Debug, PartialEq)]
pub enum ValuePointer {
    LocalVariable(String),
    BlackboardPointer(String),
}

#[allow(clippy::should_implement_trait)]
impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::String(value) => !value.is_empty(),
            Value::Float(_) => !self.eq(&Value::Float(0.0)),
            Value::Integer(value) => *value != 0,
            Value::Boolean(value) => *value,
            Value::Empty => false,
        }
    }

    pub fn as_int(&self) -> ExprResult<i64> {
        match self {
            Value::Float(value) => Ok(*value as i64),
            Value::Integer(value) => Ok(*value),
            Value::Boolean(value) => {
                if *value {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            name => Err(anyhow::format_err!(
                "{name:?} cannot be represented as an integer"
            )),
        }
    }

    pub fn as_float(&self) -> ExprResult<f64> {
        match self {
            Value::Float(value) => Ok(*value),
            Value::Integer(value) => Ok(*value as f64),
            Value::Boolean(value) => {
                if *value {
                    Ok(1.0)
                } else {
                    Ok(0.0)
                }
            }
            name => Err(anyhow::format_err!(
                "{name:?} cannot be represented as a float"
            )),
        }
    }

    pub fn checked_add(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Addition is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Addition is not allowed for {other:?}"));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value + *other,
                    Value::Integer(other) => *value + (*other as f64),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 + *other),
                    Value::Integer(other) => {
                        Value::Integer(value.checked_add(*other).ok_or_else(|| {
                            anyhow::format_err!("Integer overflow during addition")
                        })?)
                    }
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn checked_sub(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!(
                "Subtraction is not allowed for {self:?}"
            ));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!(
                "Subtraction is not allowed for {other:?}"
            ));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value - *other,
                    Value::Integer(other) => *value - (*other as f64),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 - *other),
                    Value::Integer(other) => {
                        Value::Integer(value.checked_sub(*other).ok_or_else(|| {
                            anyhow::format_err!("Integer overflow during subtraction")
                        })?)
                    }
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn checked_mul(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!(
                "Multiplication is not allowed for {self:?}"
            ));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!(
                "Multiplication is not allowed for {other:?}"
            ));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value * *other,
                    Value::Integer(other) => *value * (*other as f64),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 * *other),
                    Value::Integer(other) => {
                        Value::Integer(value.checked_mul(*other).ok_or_else(|| {
                            anyhow::format_err!("Integer overflow during multiplication")
                        })?)
                    }
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn checked_div(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Division is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Division is not allowed for {other:?}"));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value / *other,
                    Value::Integer(other) => *value / (*other as f64),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 / *other),
                    Value::Integer(other) => {
                        Value::Integer(value.checked_div(*other).ok_or_else(|| {
                            anyhow::format_err!("Integer overflow during division")
                        })?)
                    }
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn checked_mod(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {other:?}"));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value % *other,
                    Value::Integer(other) => *value % (*other as f64),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 % *other),
                    Value::Integer(other) => Value::Integer(
                        value
                            .checked_rem(*other)
                            .ok_or_else(|| anyhow::format_err!("Integer overflow during modulo"))?,
                    ),
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn checked_pow(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {other:?}"));
        }

        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => value.powf(*other),
                    Value::Integer(other) => value.powi(i32::try_from(*other)?),
                    _ => unreachable!(),
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float((*value as f64).powf(*other)),
                    Value::Integer(other) => {
                        Value::Integer(value.checked_pow(u32::try_from(*other)?).ok_or_else(
                            || anyhow::format_err!("Integer overflow during exponentiation"),
                        )?)
                    }
                    _ => unreachable!(),
                };

                Ok(value)
            }
            _ => unreachable!(),
        }
    }

    pub fn eq(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Value::Float(value) => match other {
                Value::Integer(other) => f64::abs(*value - *other as f64) < f64::EPSILON,
                Value::Float(other) => f64::abs(*value - *other) < f64::EPSILON,
                Value::Boolean(other) => {
                    let bool_val = if *other { 1 } else { 0 };
                    f64::abs(*value - bool_val as f64) < f64::EPSILON
                }
                _ => false,
            },
            Value::Integer(value) => match other {
                Value::Integer(other) => *value == *other,
                Value::Float(other) => f64::abs(*value as f64 - *other) < f64::EPSILON,
                Value::Boolean(other) => {
                    let bool_val = if *other { 1 } else { 0 };
                    *value == bool_val
                }
                _ => false,
            },
            Value::Boolean(value) => *value == other.is_truthy(),
            Value::Empty => matches!(other, Value::Empty),
        }
    }

    pub fn neq(&self, other: &Value) -> bool {
        !self.eq(other)
    }

    pub fn gt(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value > other
                } else {
                    false
                }
            }
            Value::Float(value) => match other {
                Value::Integer(other_val) => self.neq(other) && *value > (*other_val as f64),
                Value::Float(other_val) => self.neq(other) && *value > *other_val,
                Value::Boolean(other_val) => {
                    let bool_val = if *other_val { 1 } else { 0 };
                    self.neq(other) && *value > bool_val as f64
                }
                _ => false,
            },
            Value::Integer(value) => match other {
                Value::Integer(other) => *value > *other,
                Value::Float(other_val) => self.neq(other) && (*value as f64) > *other_val,
                Value::Boolean(other_val) => {
                    let bool_val = if *other_val { 1 } else { 0 };
                    self.neq(other) && *value > bool_val
                }
                _ => false,
            },
            Value::Boolean(value) => {
                if let Value::Boolean(other) = other {
                    let value = if *value { 1 } else { 0 };
                    let other = if *other { 1 } else { 0 };

                    value > other
                } else {
                    false
                }
            }
            Value::Empty => false,
        }
    }

    pub fn lt(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value < other
                } else {
                    false
                }
            }
            Value::Float(value) => match other {
                Value::Integer(other_val) => self.neq(other) && *value < (*other_val as f64),
                Value::Float(other_val) => self.neq(other) && *value < *other_val,
                Value::Boolean(other_val) => {
                    let bool_val = if *other_val { 1 } else { 0 };
                    self.neq(other) && *value < bool_val as f64
                }
                _ => false,
            },
            Value::Integer(value) => match other {
                Value::Integer(other) => *value < *other,
                Value::Float(other_val) => self.neq(other) && (*value as f64) < *other_val,
                Value::Boolean(other_val) => {
                    let bool_val = if *other_val { 1 } else { 0 };
                    self.neq(other) && *value < bool_val
                }
                _ => false,
            },
            Value::Boolean(value) => {
                if let Value::Boolean(other) = other {
                    let value = if *value { 1 } else { 0 };
                    let other = if *other { 1 } else { 0 };

                    value < other
                } else {
                    false
                }
            }
            Value::Empty => false,
        }
    }

    pub fn geq(&self, other: &Value) -> bool {
        self.gt(other) || self.eq(other)
    }

    pub fn leq(&self, other: &Value) -> bool {
        self.lt(other) || self.eq(other)
    }

    pub fn not(&self) -> bool {
        !self.is_truthy()
    }

    pub fn neg(&self) -> ExprResult<Value> {
        match self {
            Value::Float(value) => Ok(Value::Float(-*value)),
            Value::Integer(value) => Ok(Value::Integer(-*value)),
            Value::Boolean(_) => Ok(Value::Integer(-self.as_int()?)),
            name => Err(anyhow::format_err!(
                "Negation cannot be performed on {name:?}"
            )),
        }
    }

    pub fn and(&self, other: &Value) -> bool {
        self.is_truthy() && other.is_truthy()
    }

    pub fn or(&self, other: &Value) -> bool {
        self.is_truthy() || other.is_truthy()
    }
}

impl Value {
    pub fn as_type(&self) -> ValueType {
        match self {
            Value::String(_) => ValueType::String,
            Value::Float(_) => ValueType::Float,
            Value::Integer(_) => ValueType::Integer,
            Value::Boolean(_) => ValueType::Boolean,
            Value::Empty => ValueType::Empty,
        }
    }
}

impl ValuePointer {
    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Self::LocalVariable(name) => Some(name),
            _ => None,
        }
    }
}

macro_rules! impl_into_value {
    ($var:ident as $ty:ty => $($from_ty:ty),+) => {
        $(
            impl From<$from_ty> for Value {
                fn from(value: $from_ty) -> Self {
                    Value::$var(value as $ty)
                }
            }
        )+
    };
}

impl_into_value!(Integer as i64 => i64, u32, i32, u16, i16, u8, i8);
impl_into_value!(Float as f64 => f32, f64);

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl From<()> for Value {
    fn from(_value: ()) -> Self {
        Value::Empty
    }
}
