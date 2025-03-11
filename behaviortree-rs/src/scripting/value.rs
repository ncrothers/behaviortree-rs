#[derive(Debug, Clone)]
pub enum Value {
    /// A string value.
    String(String),
    /// A float value.
    Float(f64),
    /// An integer value.
    Int(i64),
    /// A boolean value.
    Boolean(bool),
    /// An empty value.
    Empty,
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => f64::abs(l0 - r0) < f64::EPSILON,
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Value {}
