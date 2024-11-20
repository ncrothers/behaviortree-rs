use behaviortree_rs_derive::bt_node;
use evalexpr::{
    build_operator_tree, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value,
};

use crate::{
    basic_types::NodeStatus,
    blackboard::BlackboardString,
    macros::{define_ports, input_port},
    nodes::{NodeError, NodeResult, TreeNodeData},
};

/// Define a conditional expression in a port, resulting in the node returning
/// either `Success` or `Failure` based on the expression result.
#[bt_node(SyncActionNode)]
pub struct ConditionNode {
    #[bt(default)]
    expr: Option<evalexpr::Node>,
}

#[bt_node(SyncActionNode)]
impl ConditionNode {
    fn ports() -> crate::basic_types::PortsList {
        define_ports!(input_port!("expr", expr))
    }

    async fn tick(&mut self) -> NodeResult {
        if self.run_condition(node_)? {
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn run_condition(&mut self, node: &mut TreeNodeData) -> NodeResult<bool> {
        if self.expr.is_none() {
            let expr_str = node
                .config
                .input_ports
                .get("expr")
                .expect("couldn't get expr port, shouldn't be possible");
            self.compile_condition(expr_str);
        }

        let expr = self
            .expr
            .as_ref()
            .expect("expression is None, shouldn't be possible");

        let mut context = HashMapContext::<DefaultNumericTypes>::new();

        for key in expr.iter_variable_identifiers() {
            // Check if it's a blackboard pointer
            if let Some(inner_key) = key.strip_bb_pointer() {
                let (name, var_type) = inner_key
                    .split_once(':')
                    .expect("variable missing : delimiter, shouldn't be possible");

                let value = match var_type {
                    "int" => {
                        Value::Int(node.config.blackboard.get::<i64>(name).ok_or_else(|| {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as an integer"
                            ))
                        })?)
                    }
                    "float" => {
                        Value::Float(node.config.blackboard.get::<f64>(name).ok_or_else(|| {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as a float"
                            ))
                        })?)
                    }
                    "str" => Value::String(node.config.blackboard.get::<String>(name).ok_or_else(
                        || {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as a string"
                            ))
                        },
                    )?),
                    "bool" => Value::Boolean(node.config.blackboard.get::<bool>(name).ok_or_else(
                        || {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as a bool"
                            ))
                        },
                    )?),
                    _ => unreachable!(),
                };

                context
                    .set_value(key.to_owned(), value)
                    .map_err(|e| NodeError::ConditionExpressionError(e.to_string()))?;
            }
        }

        let res = expr
            .eval_boolean_with_context(&context)
            .map_err(|e| NodeError::ConditionExpressionError(e.to_string()))?;

        Ok(res)
    }

    fn compile_condition(&mut self, condition: &str) {
        self.expr = Some(
            build_operator_tree::<DefaultNumericTypes>(condition)
                .expect("couldn't compile expression; this shouldn't happen"),
        );
    }

    async fn halt(&mut self) {
        node_.reset_child().await;
    }
}
