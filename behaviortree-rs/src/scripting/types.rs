use super::operator::Operator;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
    pub(super) operator: Operator,
    pub(super) children: Vec<Node>,
}