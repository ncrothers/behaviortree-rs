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

impl Operator {
    pub(crate) fn is_atomic(&self) -> bool {
        matches!(self, Self::VariableIdentifierRead { .. }
                | Self::BlackboardKeyIdentifierRead { .. }
                | Self::Const { .. }
                | Self::FunctionIdentifier { .. }
                | Self::RootNode)
    }

    pub(crate) fn is_unary(&self) -> bool {
        matches!(self, Self::Neg | Self::Not)
    }

    pub(crate) fn is_binary(&self) -> bool {
        !self.is_unary() && !self.is_atomic() && !matches!(self, Self::Chain | Self::VariableIdentifierWrite { .. } | Self::BlackboardKeyIdentifierWrite { .. })
    }

    /// Returns the precedence of the operator.
    /// A high precedence means that the operator has priority to be deeper in the tree.
    pub(crate) const fn precedence(&self) -> i32 {
        use Operator::*;
        match self {
            RootNode => 200,

            Add | Sub => 95,
            Neg => 110,
            Mul | Div | Mod => 100,
            Exp => 120,

            Eq | Neq | Gt | Lt | Geq | Leq => 80,
            And => 75,
            Or => 70,
            Not => 110,

            Assign => 50,
            Walrus => 50,

            Chain => 0,

            Const { .. } => 200,

            VariableIdentifierWrite { .. }
            | VariableIdentifierRead { .. }
            | BlackboardKeyIdentifierWrite { .. }
            | BlackboardKeyIdentifierRead { .. } => 200,

            FunctionIdentifier { .. } => 190,
        }
    }
}
