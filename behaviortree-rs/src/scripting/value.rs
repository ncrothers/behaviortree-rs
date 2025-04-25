use crate::blackboard::EntryRef;

use super::expr::ExprResult;

/// Wrapper around the valid types for expression operations.
#[derive(Debug, Clone)]
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
pub(crate) enum ValueType {
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
    pub fn is_str(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Self::Integer(_))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Return the `Value` as a `&str` if the inner type is [`Value::String`].
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let val = Value::String("foo".into());
    /// assert_eq!(val.as_str(), Some("foo"));
    ///
    /// let val = Value::Integer(10);
    /// assert_eq!(val.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(val) = self {
            Some(val)
        } else {
            None
        }
    }

    /// Return the `Value` as a `f64` if the inner type is [`Value::Float`].
    ///
    /// If you want to instead try to _convert_ the value into a `f64`, use
    /// [`Value::cast_to_float`].
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let val = Value::Float(10.0);
    /// assert_eq!(val.as_float(), Some(10.0));
    ///
    /// let val = Value::Integer(10);
    /// assert_eq!(val.as_float(), None);
    /// ```
    pub fn as_float(&self) -> Option<f64> {
        if let Self::Float(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    /// Return the `Value` as a `i64` if the inner type is [`Value::Integer`].
    ///
    /// If you want to instead try to _convert_ the value into a `i64`, use
    /// [`Value::cast_to_int`].
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let val = Value::Integer(10);
    /// assert_eq!(val.as_int(), Some(10));
    ///
    /// let val = Value::Boolean(true);
    /// assert_eq!(val.as_int(), None);
    /// ```
    pub fn as_int(&self) -> Option<i64> {
        if let Self::Integer(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    /// Return the `Value` as a `bool` if the inner type is [`Value::Boolean`].
    ///
    /// If you want to instead _convert_ the value into a `bool`, use
    /// [`Value::is_truthy`].
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// let val = Value::Boolean(true);
    /// assert_eq!(val.as_bool(), Some(true));
    ///
    /// let val = Value::Integer(10);
    /// assert_eq!(val.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Boolean(val) = self {
            Some(*val)
        } else {
            None
        }
    }

    pub fn as_empty(&self) -> Option<()> {
        if let Self::Empty = self {
            Some(())
        } else {
            None
        }
    }

    /// Interprets the value as a `bool` to determine its "truthiness".
    ///
    /// # Truthiness Rules
    ///
    /// The following cases will return `false`, all other cases return `true`:
    ///
    /// - `Value::String`
    ///     - `""` (empty string)
    /// - `Value::Float`
    ///     - `0.0`
    ///     - `f64::NAN` (NaN)
    /// - `Value::Integer`
    ///     - `0`
    /// - `Value::Boolean`
    ///     - `false`
    /// - `Value::Empty` (always false)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::String(value) => !value.is_empty(),
            // Both 0.0 and NaN are false
            Value::Float(value) => !(float_eq(*value, 0.0) || value.is_nan()),
            Value::Integer(value) => *value != 0,
            Value::Boolean(value) => *value,
            Value::Empty => false,
        }
    }

    /// Returns the inner type as `i64` if it can be converted to the type.
    ///
    /// - `Value::Integer`: value as-is
    /// - `Value::Boolean`:
    ///     - `true => 1`
    ///     - `false => 0`
    pub(crate) fn cast_to_int(&self) -> Option<i64> {
        match self {
            Value::Integer(value) => Some(*value),
            Value::Boolean(value) => {
                if *value {
                    Some(1)
                } else {
                    Some(0)
                }
            }
            _ => None,
        }
    }

    /// Returns the inner type as `f64` if it can be converted to the type.
    ///
    /// - `Value::Float`: value as-is
    /// - `Value::Integer`: converted to float using `as f64`
    /// - `Value::Boolean`:
    ///     - `true => 1.0`
    ///     - `false => 0.0`
    pub(crate) fn cast_to_float(&self) -> Option<f64> {
        match self {
            Value::Float(value) => Some(*value),
            Value::Integer(value) => Some(*value as f64),
            Value::Boolean(value) => {
                if *value {
                    Some(1.0)
                } else {
                    Some(0.0)
                }
            }
            _ => None,
        }
    }

    /// Add `Value`s together, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// or if there is integer overflow.
    pub(crate) fn checked_add(&self, other: &Value) -> ExprResult<Value> {
        expect_string_number_bool(self)?;
        expect_string_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            Ok(Value::String([lhs, rhs].concat()))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            Ok(Value::Integer(lhs.checked_add(rhs).ok_or_else(|| {
                anyhow::format_err!("Integer overflow during addition")
            })?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs + rhs))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for add: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Subtract `other`, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// or if there is integer overflow.
    pub(crate) fn checked_sub(&self, other: &Value) -> ExprResult<Value> {
        expect_number_bool(self)?;
        expect_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            Ok(Value::Integer(lhs.checked_sub(rhs).ok_or_else(|| {
                anyhow::format_err!("Integer overflow during subtraction")
            })?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs - rhs))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for sub: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Multiply `Value`s together, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// or if there is integer overflow.
    pub(crate) fn checked_mul(&self, other: &Value) -> ExprResult<Value> {
        expect_number_bool(self)?;
        expect_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            Ok(Value::Integer(lhs.checked_mul(rhs).ok_or_else(|| {
                anyhow::format_err!("Integer overflow during multiplication")
            })?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs * rhs))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for mul: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Divide by `other`, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// if there is integer overflow, or if `other` is `0`.
    pub(crate) fn checked_div(&self, other: &Value) -> ExprResult<Value> {
        expect_number_bool(self)?;
        expect_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            Ok(Value::Integer(lhs.checked_div(rhs).ok_or_else(|| {
                anyhow::format_err!("Integer overflow during division")
            })?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs / rhs))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for div: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Calculate remainder by `other`, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// if there is integer overflow, or if `other` is `0`.
    pub(crate) fn checked_mod(&self, other: &Value) -> ExprResult<Value> {
        expect_number_bool(self)?;
        expect_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            Ok(Value::Integer(lhs.checked_rem(rhs).ok_or_else(|| {
                anyhow::format_err!("Integer overflow during remainder")
            })?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs % rhs))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for rem: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Exponentiate by `other`, only working if the underlying types are numeric.
    /// Returns `Err` if either value is not `Value::Integer` or `Value::Float`,
    /// if there is integer overflow.
    pub(crate) fn checked_pow(&self, other: &Value) -> ExprResult<Value> {
        /// Return an error if the exponent is too big
        fn check_exponent(val: i64) -> ExprResult<()> {
            if val > u32::MAX as i64 {
                Err(anyhow::format_err!(
                    "Exponent too big, exceeds u32::MAX: {val}"
                ))
            } else {
                Ok(())
            }
        }

        expect_number_bool(self)?;
        expect_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.cast_to_int(), other.cast_to_int()) {
            check_exponent(rhs)?;

            Ok(Value::Integer(lhs.checked_pow(rhs as u32).ok_or_else(
                || anyhow::format_err!("Integer overflow during remainder"),
            )?))
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            Ok(Value::Float(lhs.powf(rhs)))
        } else {
            Err(anyhow::format_err!(
                "Mismatched types for rem: {:?} + {:?}",
                self.as_type(),
                other.as_type()
            ))
        }
    }

    /// Evaluate equality with `other`.
    pub(crate) fn _eq(&self, other: &Value) -> bool {
        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            lhs == rhs
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            lhs == rhs
        } else if let (Some(lhs), Some(rhs)) = (self.as_bool(), other.as_bool()) {
            lhs == rhs
        } else if let (Some(_), Some(_)) = (self.as_empty(), other.as_empty()) {
            true
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            float_eq(lhs, rhs)
        } else {
            false
        }
    }

    /// Negation of [`Value::_eq`].
    pub fn _neq(&self, other: &Value) -> bool {
        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            lhs != rhs
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            lhs != rhs
        } else if let (Some(lhs), Some(rhs)) = (self.as_bool(), other.as_bool()) {
            lhs != rhs
        } else if let (Some(_), Some(_)) = (self.as_empty(), other.as_empty()) {
            false
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            float_neq(lhs, rhs)
        } else {
            true
        }
    }

    pub(crate) fn _gt(&self, other: &Value) -> ExprResult<bool> {
        expect_string_number_bool(self)?;
        expect_string_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            Ok(lhs > rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            Ok(lhs > rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            // Check greater than and ensure they aren't within epsilon
            Ok(lhs > rhs && float_neq(lhs, rhs))
        } else {
            unreachable!("Values should have been converted to float")
        }
    }

    pub(crate) fn _lt(&self, other: &Value) -> ExprResult<bool> {
        expect_string_number_bool(self)?;
        expect_string_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            Ok(lhs < rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            Ok(lhs < rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            // Check greater than and ensure they aren't within epsilon
            Ok(lhs < rhs && float_neq(lhs, rhs))
        } else {
            unreachable!("Values should have been converted to float")
        }
    }

    pub(crate) fn _geq(&self, other: &Value) -> ExprResult<bool> {
        expect_string_number_bool(self)?;
        expect_string_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            Ok(lhs >= rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            Ok(lhs >= rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            // Check greater than and ensure they aren't within epsilon
            Ok(lhs >= rhs || float_eq(lhs, rhs))
        } else {
            unreachable!("Values should have been converted to float")
        }
    }

    pub(crate) fn _leq(&self, other: &Value) -> ExprResult<bool> {
        expect_string_number_bool(self)?;
        expect_string_number_bool(other)?;

        if let (Some(lhs), Some(rhs)) = (self.as_str(), other.as_str()) {
            Ok(lhs <= rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.as_int(), other.as_int()) {
            Ok(lhs <= rhs)
        } else if let (Some(lhs), Some(rhs)) = (self.cast_to_float(), other.cast_to_float()) {
            // Check greater than and ensure they aren't within epsilon
            Ok(lhs <= rhs || float_eq(lhs, rhs))
        } else {
            unreachable!("Values should have been converted to float")
        }
    }

    pub(crate) fn _not(&self) -> bool {
        !self.is_truthy()
    }

    pub(crate) fn _neg(&self) -> ExprResult<Value> {
        expect_number_bool(self)?;

        match self {
            Value::Float(value) => Ok(Value::Float(-*value)),
            Value::Integer(value) => Ok(Value::Integer(-*value)),
            Value::Boolean(_) => Ok(Value::Integer(
                -self
                    .cast_to_int()
                    .expect("boolean should always be castable to int"),
            )),
            _ => unreachable!("condition already checked for"),
        }
    }

    pub(crate) fn _and(&self, other: &Value) -> bool {
        self.is_truthy() && other.is_truthy()
    }

    pub(crate) fn _or(&self, other: &Value) -> bool {
        self.is_truthy() || other.is_truthy()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self._eq(other)
    }
}

impl Value {
    pub(crate) fn as_type(&self) -> ValueType {
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

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.into())
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

fn float_eq(lhs: f64, rhs: f64) -> bool {
    lhs == rhs || f64::abs(lhs - rhs) < f64::EPSILON
}

fn float_neq(lhs: f64, rhs: f64) -> bool {
    f64::abs(lhs - rhs) >= f64::EPSILON
}

fn expect_string_number_bool(value: &Value) -> ExprResult<()> {
    if !matches!(
        value.as_type(),
        ValueType::String | ValueType::Float | ValueType::Integer | ValueType::Boolean
    ) {
        Err(anyhow::format_err!(
            "Expected a string, number, or boolean. Found: {value:?}"
        ))
    } else {
        Ok(())
    }
}

fn expect_number_bool(value: &Value) -> ExprResult<()> {
    if !matches!(
        value.as_type(),
        ValueType::Float | ValueType::Integer | ValueType::Boolean
    ) {
        Err(anyhow::format_err!(
            "Expected a number or boolean. Found: {value:?}"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case((Value::from(10), Value::from(10)), Ok(Value::from(20)))]
    #[case((Value::from(10.0), Value::from(10)), Ok(Value::from(20.0)))]
    #[case((Value::from("foo"), Value::from("bar")), Ok(Value::from("foobar")))]
    #[case((Value::from(10), Value::from(true)), Ok(Value::from(11)))]
    fn add(#[case] value: (Value, Value), #[case] output: ExprResult<Value>) {
        let res = value.0.checked_add(&value.1);

        match output {
            Ok(output) => {
                assert!(res.is_ok());

                let res = res.unwrap();

                assert_eq!(res, output);
            }
            Err(_) => {
                assert!(res.is_err());
            }
        }
    }

    #[rstest]
    #[case::string(Value::String("".into()), false)]
    #[case::string(Value::String("foo".into()), true)]
    #[case::float(Value::Float(0.0), false)]
    #[case::float(Value::Float(10.0 - 2.0 * 5.0), false)]
    #[case::float(Value::Float(f64::NAN), false)]
    #[case::float(Value::Float(-10.0), true)]
    #[case::int(Value::Integer(0), false)]
    #[case::int(Value::Integer(-10), true)]
    #[case::bool(Value::Boolean(true), true)]
    #[case::bool(Value::Boolean(false), false)]
    #[case::empty(Value::Empty, false)]
    fn is_truthy(#[case] value: Value, #[case] output: bool) {
        assert_eq!(value.is_truthy(), output);
    }

    #[rstest]
    #[case::bool(Value::Boolean(true), Some(1))]
    #[case::bool(Value::Boolean(false), Some(0))]
    #[case::nan(Value::Float(1.0), None)]
    #[case::string(Value::String("".into()), None)]
    #[case::empty(Value::Empty, None)]
    fn cast_to_int(#[case] value: Value, #[case] output: Option<i64>) {
        assert_eq!(value.cast_to_int(), output);
    }
}
