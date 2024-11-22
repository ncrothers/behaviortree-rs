use std::any::Any;

pub trait BlackboardValue: Any + std::fmt::Debug {
    /// Returns the string representation of the value. This method should return
    /// `None` if the value is not representable as a string.
    fn as_string(&self) -> Option<String>;
}

#[derive(Debug)]
pub enum Value {
    /// Integer-based value
    Integer(i64),
    /// Float-based value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Collection of `Value`s
    Vec(Vec<Value>),
    /// Custom types
    Dynamic(Box<dyn BlackboardValue>),
}

impl Value {
    /// Returns the string representation of the value. This method will return `None`
    /// if it's a [`Value::Dynamic`] type whose [`BlackboardValue::as_string`] method
    /// returns `None`.
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::Integer(value) => Some(value.to_string()),
            Value::Float(value) => Some(value.to_string()),
            Value::String(value) => Some(value.clone()),
            Value::Boolean(value) => Some(value.to_string()),
            Value::Vec(vec) => {
                let mut output = String::from("[");

                for (i, item) in vec.iter().enumerate() {
                    let as_str = item.as_string()?;
                    output.reserve(as_str.len() + 1);
                    output.push_str(&as_str);

                    if i < vec.len() - 1 {
                        output.push(',');
                    }
                }

                output.push(']');

                Some(output)
            }
            Value::Dynamic(blackboard_value) => blackboard_value.as_string(),
        }
    }
}

macro_rules! impl_from_int {
    ($($int:ty)+) => {
        $(
            impl From<$int> for Value {
                fn from(value: $int) -> Value {
                    Value::Integer(value as i64)
                }
            }
        )*
    };
}

impl_from_int! { i8 u8 i16 u16 i32 u32 i64 u64 i128 u128 }

impl<T> From<T> for Value
where
    T: BlackboardValue + 'static,
{
    fn from(value: T) -> Self {
        Value::Dynamic(Box::new(value))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::integer(Value::Integer(128), Some("128"))]
    #[case::float(Value::Float(1.23), Some("1.23"))]
    #[case::string(Value::String("hello".into()), Some("hello"))]
    #[case::boolean(Value::Boolean(true), Some("true"))]
    #[case::vec_ints(Value::Vec(vec![1.into(), 2.into(), 3.into()]), Some("[1,2,3]"))]
    fn as_string(#[case] value: Value, #[case] output: Option<&str>) {
        assert_eq!(value.as_string().as_deref(), output);
    }
}
