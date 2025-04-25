use std::{collections::HashMap, ops::Deref, sync::OnceLock};

use pest::{
    iterators::{Pair, Pairs},
    pratt_parser::{PrattParser, PrattParserMap},
    Parser,
};
use pest_derive::Parser;

use super::{
    context::ContextInternal,
    operator::Op,
    value::{Value, ValueOrAny, ValuePointer},
    Context,
};

#[derive(Parser)]
#[grammar = "scripting/expr.pest"]
pub struct ExprParser;

pub type ExprResult<T> = anyhow::Result<T>;

static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();

fn pratt_parser_base() -> PrattParser<Rule> {
    use pest::pratt_parser::{Assoc::*, Op};
    use Rule::*;

    // Precedence is defined lowest to highest
    PrattParser::new()
        // Assignments
        .op(Op::infix(variable_assign_s, Right)
            | Op::infix(bb_walrus_s, Right)
            | Op::infix(bb_assign_s, Right))
        // OR
        .op(Op::infix(or, Left))
        // AND
        .op(Op::infix(and, Left))
        // Comparisons
        .op(Op::infix(eq, Left)
            | Op::infix(neq, Left)
            | Op::infix(gt, Left)
            | Op::infix(geq, Left)
            | Op::infix(lt, Left)
            | Op::infix(leq, Left))
        // Addition/subtraction
        .op(Op::infix(add, Left) | Op::infix(subtract, Left))
        // Multiply/divide/modulo
        .op(Op::infix(multiply, Left) | Op::infix(divide, Left) | Op::infix(modulo, Left))
        // Negation/NOT
        .op(Op::prefix(neg) | Op::prefix(not))
        // Exponentiation
        .op(Op::infix(exponentiation, Left))
}

fn pratt_parser<'pratt, 'a, 'b>(
    pratt: &'pratt PrattParser<Rule>,
) -> PrattParserMap<'pratt, 'a, 'b, Rule, impl FnMut(Pair<'b, Rule>) -> Expr + 'pratt, Expr> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::integer => Expr::Value(Value::Integer(primary.as_str().parse::<i64>().unwrap())),
            Rule::float => Expr::Value(Value::Float(primary.as_str().parse::<f64>().unwrap())),
            Rule::string => Expr::Value(Value::String(
                primary
                    .into_inner()
                    .find_first_tagged("name")
                    .expect("string should have a \"name\"-tagged child")
                    .as_str()
                    .into(),
            )),
            Rule::boolean => Expr::Value(Value::Boolean(primary.as_str() == "true")),
            Rule::variable => {
                Expr::ValuePointer(ValuePointer::LocalVariable(primary.as_str().into()))
            }
            Rule::bb_pointer => {
                let inner = primary.into_inner();
                let name = inner
                    .find_first_tagged("name")
                    .expect("tag will exist")
                    .as_str()
                    .into();
                Expr::ValuePointer(ValuePointer::BlackboardPointer(name))
            }
            Rule::function_call => {
                let inner = primary.into_inner();
                let args = inner
                    .clone()
                    .find(|pair| pair.as_rule() == Rule::function_args)
                    .unwrap();
                let args = args
                    .into_inner()
                    .map(|arg| pratt_parser(pratt).parse(arg.into_inner()))
                    .collect();

                let name = inner.find_first_tagged("name").unwrap().as_str();
                Expr::FunctionCall {
                    name: name.into(),
                    args,
                }
            }
            Rule::function_arg_single => pratt_parser(pratt).parse(primary.into_inner()),
            Rule::value_expr => pratt_parser(pratt).parse(primary.into_inner()),
            rule => unreachable!("Expr::parse expected atom, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let op = match op.as_rule() {
                Rule::add => Op::Add,
                Rule::subtract => Op::Sub,
                Rule::multiply => Op::Mul,
                Rule::divide => Op::Div,
                Rule::modulo => Op::Mod,
                Rule::exponentiation => Op::Exp,
                Rule::eq => Op::Eq,
                Rule::neq => Op::Neq,
                Rule::gt => Op::Gt,
                Rule::lt => Op::Lt,
                Rule::geq => Op::Geq,
                Rule::leq => Op::Leq,
                Rule::and => Op::And,
                Rule::or => Op::Or,
                Rule::variable_assign_s => Op::VariableAssign,
                Rule::bb_assign_s => Op::BlackboardAssign { create: false },
                Rule::bb_walrus_s => Op::BlackboardAssign { create: true },
                rule => unreachable!("Expr::parse expected infix operation, found {:?}", rule),
            };
            Expr::BinaryOp {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            }
        })
        .map_prefix(|op, rhs| {
            let op = match op.as_rule() {
                Rule::neg => Op::Neg,
                Rule::not => Op::Not,
                rule => unreachable!("Expr::parse expected prefix operation, found {:?}", rule),
            };

            Expr::UnaryOp {
                op,
                value: Box::new(rhs),
            }
        })
}

#[derive(Debug, PartialEq)]
pub enum Expr {
    Value(Value),
    ValuePointer(ValuePointer),
    Chain(Vec<Expr>),
    BinaryOp {
        lhs: Box<Expr>,
        op: Op,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: Op,
        value: Box<Expr>,
    },
    FunctionCall {
        name: String,
        args: Vec<Expr>,
    },
}

impl Expr {
    /// Parse a string into an `Expr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::scripting::Expr;
    ///
    /// let expr_str = "1 + 2";
    ///
    /// let expr = Expr::parse(expr_str);
    ///
    /// assert!(expr.is_ok());
    /// ```
    pub fn parse(text: &str) -> anyhow::Result<Expr> {
        let mut statements = text
            .split(';')
            .map(|stmt| {
                if stmt.is_empty() {
                    Ok(Expr::Value(Value::Empty))
                } else {
                    let stmt = stmt.trim();
                    let pairs = lex_expr(stmt)?.filter(|pair| pair.as_rule() != Rule::EOI);

                    Ok(pairs_to_expr(pairs))
                }
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let expr = if statements.len() > 1 {
            Expr::Chain(statements)
        } else if statements.is_empty() {
            return Err(anyhow::format_err!("No expressions found"));
        } else {
            statements.pop().unwrap()
        };

        expr.validate()?;

        Ok(expr)
    }

    pub(crate) fn validate(&self) -> anyhow::Result<()> {
        let mut variables = Vec::new();

        Self::validate_recursive(self, &mut variables)
    }

    pub(crate) fn as_value_pointer(&self) -> Option<&ValuePointer> {
        match self {
            Self::ValuePointer(value) => Some(value),
            _ => None,
        }
    }

    fn validate_recursive<'a>(expr: &'a Expr, variables: &mut Vec<&'a str>) -> anyhow::Result<()> {
        match expr {
            Expr::ValuePointer(value_pointer) => match value_pointer {
                ValuePointer::LocalVariable(name) => {
                    if !variables.contains(&name.as_str()) {
                        return Err(anyhow::format_err!("Use of undefined variable {name}"));
                    }
                }
                ValuePointer::BlackboardPointer(_) => (),
            },
            Expr::Chain(exprs) => {
                for expr in exprs {
                    Self::validate_recursive(expr, variables)?;
                }
            }
            Expr::BinaryOp { lhs, op, rhs } => {
                Self::validate_recursive(rhs, variables)?;

                if op == &Op::VariableAssign {
                    let name = lhs
                        .as_value_pointer()
                        .ok_or_else(|| anyhow::format_err!("LHS of a variable assignment isn't a ValuePointer, shouldn't be possible."))?
                        .as_variable()
                        .ok_or_else(|| anyhow::format_err!("LHS ValuePointer of variable assignment isn't a LocalVariable, shouldn't be possible."))?;

                    variables.push(name);
                } else {
                    Self::validate_recursive(lhs, variables)?;
                }
            }
            Expr::UnaryOp { op, value } => {
                match op {
                    Op::Neg => {
                        // Negation operator not allowed on string or boolean literals
                        match value.deref() {
                            Expr::Value(Value::String(_)) => {
                                return Err(anyhow::format_err!(
                                    "Negation operator not allowed on strings."
                                ))
                            }
                            Expr::Value(Value::Boolean(_)) => {
                                return Err(anyhow::format_err!(
                                    "Negation operator not allowed on booleans."
                                ))
                            }
                            _ => (),
                        }
                    }
                    Op::Not => {
                        // Negation operator not allowed on string or boolean literals
                        match value.deref() {
                            Expr::Value(Value::Boolean(_)) | Expr::Value(Value::Integer(_)) => (),
                            Expr::Value(_) => {
                                return Err(anyhow::format_err!(
                                    "NOT operator only allowed on booleans and integers."
                                ))
                            }
                            _ => (),
                        }
                    }
                    _ => return Err(anyhow::format_err!("Expected unary operator, found {op:?}")),
                }

                Self::validate_recursive(value, variables)?;
            }
            Expr::FunctionCall { name: _, args } => {
                for arg in args {
                    Self::validate_recursive(arg, variables)?;
                }
            }
            Expr::Value(_) => (),
        }

        Ok(())
    }

    pub fn eval_with_context(&self, context: &Context) -> ExprResult<Value> {
        let mut context = ContextInternal {
            context,
            variables: HashMap::new(),
        };

        Self::eval_with_context_recursive(self, &mut context)
    }

    fn eval_with_context_recursive(
        expr: &Expr,
        context: &mut ContextInternal,
    ) -> ExprResult<Value> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),
            Expr::ValuePointer(value_pointer) => {
                match value_pointer {
                    ValuePointer::LocalVariable(name) => context
                        .context
                        .get_value(name)
                        .or_else(|| context.variables.get(name))
                        .cloned()
                        .ok_or_else(|| anyhow::format_err!("Context didn't contain variable")),
                    ValuePointer::BlackboardPointer(key) => {
                        let entry =
                            context
                                .context
                                .blackboard
                                .get_entry_ref(key)
                                .ok_or_else(|| {
                                    anyhow::format_err!("Blackboard key \"{key}\" did not exist")
                                })?;

                        // Try to downcast the type to one that can be put into a Value
                        let value = if let Some(val) = entry.downcast_clone::<Value>() {
                            val
                        } else if let Some(val) = entry.downcast_clone::<i64>() {
                            Value::Integer(val)
                        } else if let Some(val) = entry.downcast_clone::<u64>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<i32>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<u32>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<i16>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<u16>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<i8>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<u8>() {
                            Value::Integer(val as i64)
                        } else if let Some(val) = entry.downcast_clone::<bool>() {
                            Value::Boolean(val)
                        } else if let Some(val) = entry.downcast_clone::<String>() {
                            Value::String(val)
                        } else if let Some(val) = entry.downcast_clone::<f64>() {
                            Value::Float(val)
                        } else if let Some(val) = entry.downcast_clone::<f32>() {
                            Value::Float(val as f64)
                        } else if entry.downcast_clone::<()>().is_some() {
                            Value::Empty
                        } else {
                            return Err(anyhow::format_err!("Blackboard value at key \"{key}\" could not be converted into a Value"));
                        };

                        Ok(value)
                    }
                }
            }
            Expr::Chain(exprs) => {
                let mut result = Value::Empty;

                for expr in exprs {
                    result = Self::eval_with_context_recursive(expr, context)?;
                }

                Ok(result)
            }
            Expr::BinaryOp { lhs, op, rhs } => match op {
                Op::VariableAssign => {
                    if let Expr::ValuePointer(ValuePointer::LocalVariable(name)) = lhs.as_ref() {
                        let rhs = Self::eval_with_context_recursive(rhs, context)?;

                        context.variables.insert(name.clone(), rhs);
                        Ok(Value::Empty)
                    } else {
                        unreachable!("VariableAssign should have LocalVariable as LHS")
                    }
                }
                Op::BlackboardAssign { create } => {
                    if let Expr::ValuePointer(ValuePointer::BlackboardPointer(key)) = lhs.as_ref() {
                        if !create && !context.context.blackboard.contains_key(key) {
                            return Err(anyhow::format_err!("Blackboard key \"{key}\" doesn't exist, and the walrus operator wasn't used to assign"));
                        }

                        // Evaluate the right hand side
                        let rhs = Self::eval_with_context_recursive(rhs, context)?;

                        // Assign the value into the blackboard, stripping the outer Value
                        match rhs {
                            Value::String(val) => context.context.blackboard.set(key, val),
                            Value::Float(val) => context.context.blackboard.set(key, val),
                            Value::Integer(val) => context.context.blackboard.set(key, val),
                            Value::Boolean(val) => context.context.blackboard.set(key, val),
                            Value::Empty => context.context.blackboard.set(key, ()),
                        }

                        Ok(Value::Empty)
                    } else {
                        unreachable!("BlackboardAssign should have BlackboardPointer as LHS")
                    }
                }
                op => {
                    let lhs = Self::eval_with_context_recursive(lhs, context)?;
                    let rhs = Self::eval_with_context_recursive(rhs, context)?;

                    op.binary(&lhs, &rhs)
                }
            },
            Expr::UnaryOp { op, value } => {
                let value = Self::eval_with_context_recursive(value, context)?;

                op.unary(&value)
            }
            Expr::FunctionCall { name, args } => {
                let args =
                    args.iter()
                        .map(|arg| match arg {
                            Expr::ValuePointer(ValuePointer::BlackboardPointer(key)) => context
                                .context
                                .blackboard
                                .get_entry_ref(key)
                                .map(ValueOrAny::Any)
                                .ok_or_else(|| {
                                    anyhow::format_err!("Blackboard key {key} doesn't exist.")
                                }),
                            expr => Self::eval_with_context_recursive(expr, context)
                                .map(ValueOrAny::Value),
                        })
                        .collect::<ExprResult<_>>()?;

                context.context.call_function(name, args)
            }
        }
    }
}

type FunctionType = dyn Fn(Vec<ValueOrAny>) -> ExprResult<Value>;

pub struct Function {
    f: Box<FunctionType>,
}

impl Function {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(Vec<ValueOrAny>) -> ExprResult<Value> + 'static,
    {
        Self { f: Box::new(f) }
    }

    pub fn call(&self, args: Vec<ValueOrAny>) -> ExprResult<Value> {
        (self.f)(args)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function").finish()
    }
}

impl<F> From<F> for Function
where
    F: Fn(Vec<ValueOrAny>) -> ExprResult<Value> + 'static,
{
    fn from(value: F) -> Self {
        Function::new(value)
    }
}

fn lex_expr(text: &str) -> anyhow::Result<Pairs<'_, Rule>> {
    Ok(ExprParser::parse(Rule::root, text)?)
}

fn pairs_to_expr<'a>(pairs: impl Iterator<Item = Pair<'a, Rule>>) -> Expr {
    let parser = PRATT_PARSER.get_or_init(pratt_parser_base);

    pratt_parser(parser).parse(pairs)
}

#[cfg(test)]
mod tests {
    use crate::Blackboard;

    use super::*;

    use rstest::rstest;

    #[rstest]
    fn set_value() {
        let blackboard = Blackboard::new();

        let mut context = Context::new(blackboard);

        assert!(context.set_value("int", 10i32).is_ok());
        assert!(context.set_value("int", 10i8).is_ok());

        assert!(context.set_value("float", 10.0f32).is_ok());

        assert!(context.set_value("bool", true).is_ok());

        assert!(context.set_value("string", String::from("hello")).is_ok());
    }

    #[rstest]
    fn set_value_type_check() {
        let blackboard = Blackboard::new();

        let mut context = Context::new(blackboard);

        context.set_value("int", 10i32).unwrap();

        assert!(context.set_value("int", false).is_err());
    }
}
