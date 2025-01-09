use evalexpr::{
    build_operator_tree, ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Value,
};

use crate::{
    basic_types::NodeStatus,
    macros::{define_ports, input_port},
    nodes::{NodeData, NodeError, NodeResult},
};

use super::{SyncActionContext, SyncActionNode};

/// The InverterNode returns Failure on Success, and Success on Failure
#[derive(Debug, Default)]
pub struct ConditionNode {
    expr: Option<evalexpr::Node>,
}

impl ConditionNode {
    fn run_condition(&mut self, node: &mut NodeData<SyncActionContext>) -> NodeResult<bool> {
        if self.expr.is_none() {
            let expr_str = node
                .meta
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
            if key.starts_with('{') && key.ends_with('}') {
                // Remove the brackets
                let inner_key = &key[1..(key.len() - 1)];
                let (name, var_type) = inner_key
                    .split_once(':')
                    .expect("variable missing : delimiter, shouldn't be possible");

                let value = match var_type {
                    "int" => Value::Int(node.blackboard.get::<i64>(name).ok_or_else(|| {
                        NodeError::BlackboardError(format!(
                            "Couldn't load blackboard key {name} as an integer"
                        ))
                    })?),
                    "float" => Value::Float(node.blackboard.get::<f64>(name).ok_or_else(|| {
                        NodeError::BlackboardError(format!(
                            "Couldn't load blackboard key {name} as a float"
                        ))
                    })?),
                    "str" => {
                        Value::String(node.blackboard.get::<String>(name).ok_or_else(|| {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as a string"
                            ))
                        })?)
                    }
                    "bool" => {
                        Value::Boolean(node.blackboard.get::<bool>(name).ok_or_else(|| {
                            NodeError::BlackboardError(format!(
                                "Couldn't load blackboard key {name} as a bool"
                            ))
                        })?)
                    }
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
}

impl SyncActionNode for ConditionNode {
    type Context = SyncActionContext;

    fn ports(&self) -> crate::basic_types::PortsList {
        define_ports!(input_port!("expr", expr))
    }

    fn tick(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult {
        if self.run_condition(ctx)? {
            Ok(NodeStatus::Success)
        } else {
            Ok(NodeStatus::Failure)
        }
    }

    fn halt(&mut self, ctx: &mut NodeData<SyncActionContext>) -> NodeResult<()> {
        ctx.reset_status();

        Ok(())
    }
}
