use smallvec::SmallVec;

use super::operator::Operator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) operator: Operator,
    pub(super) children: Vec<Node>,
}

impl Node {
    pub fn new(operator: Operator, children: Vec<Node>) -> Self {
        Self {
            operator,
            children,
        }
    }

    pub fn empty(operator: Operator) -> Self {
        Self {
            operator,
            children: Vec::new(),
        }
    }
}