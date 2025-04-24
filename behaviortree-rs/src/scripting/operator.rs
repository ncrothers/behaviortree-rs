use super::{expr::ExprResult, value::Value};

#[derive(Debug, PartialEq, Eq)]
pub enum Op {
    /// A binary addition operator.
    Add,
    /// A binary subtraction operator.
    Sub,
    /// A unary negation operator.
    Neg,
    /// A binary multiplication operator.
    Mul,
    /// A binary division operator.
    Div,
    /// A binary modulo operator.
    Mod,
    /// A binary exponentiation operator.
    Exp,

    /// A binary equality comparator.
    Eq,
    /// A binary inequality comparator.
    Neq,
    /// A binary greater-than comparator.
    Gt,
    /// A binary lower-than comparator.
    Lt,
    /// A binary greater-than-or-equal comparator.
    Geq,
    /// A binary lower-than-or-equal comparator.
    Leq,
    /// A binary logical and operator.
    And,
    /// A binary logical or operator.
    Or,
    /// A binary logical not operator.
    Not,

    /// Assign an Expr to a local variable
    VariableAssign,
    /// Assign an Expr to a Blackboard key
    BlackboardAssign {
        /// Whether to create the BB key if it doesn't already exist. Set to `true`
        /// when using the `:=` operator, otherwise `false`.
        create: bool,
    },
}

impl Op {
    pub(super) fn binary(&self, lhs: &Value, rhs: &Value) -> ExprResult<Value> {
        match self {
            Op::Add => lhs.checked_add(rhs),
            Op::Sub => lhs.checked_sub(rhs),
            Op::Mul => lhs.checked_mul(rhs),
            Op::Div => lhs.checked_div(rhs),
            Op::Mod => lhs.checked_mod(rhs),
            Op::Exp => lhs.checked_pow(rhs),
            Op::Eq => Ok(Value::Boolean(lhs.eq(rhs))),
            Op::Neq => Ok(Value::Boolean(lhs.neq(rhs))),
            Op::Gt => Ok(Value::Boolean(lhs.gt(rhs))),
            Op::Lt => Ok(Value::Boolean(lhs.lt(rhs))),
            Op::Geq => Ok(Value::Boolean(lhs.geq(rhs))),
            Op::Leq => Ok(Value::Boolean(lhs.leq(rhs))),
            Op::And => Ok(Value::Boolean(lhs.and(rhs))),
            Op::Or => Ok(Value::Boolean(lhs.or(rhs))),
            name => unreachable!("expected a binary operator, got {name:?}"),
        }
    }

    pub(super) fn unary(&self, value: &Value) -> ExprResult<Value> {
        match self {
            Op::Neg => value.neg(),
            Op::Not => Ok(Value::Boolean(value.not())),
            name => unreachable!("expected a unary operator, got {name:?}"),
        }
    }
}
