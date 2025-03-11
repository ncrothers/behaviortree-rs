use super::value::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Operator {
    RootNode,

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

    /// A binary assignment operator.
    Assign,
    /// TODO
    Walrus,

    /// An n-ary subexpression chain.
    Chain,

    /// A constant value
    Const {
        value: Value,
    },

    /// Identifier of a local variable
    VariableIdentifierRead {
        identifier: String,
    },
    /// Identifier of a local variable
    VariableIdentifierWrite {
        identifier: String,
    },
    /// Identifier of a blackboard pointer
    BlackboardKeyIdentifierRead {
        identifier: String,
    },
    /// Identifier of a blackboard pointer
    BlackboardKeyIdentifierWrite {
        identifier: String,
    },
    /// A function identifier
    FunctionIdentifier {
        identifier: String,
    },
}