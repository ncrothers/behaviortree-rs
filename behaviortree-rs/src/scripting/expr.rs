use std::{any::Any, collections::HashMap, ops::Deref, sync::OnceLock};

use pest::{
    Parser,
    iterators::{Pair, Pairs},
    pratt_parser::{PrattParser, PrattParserMap},
};
use pest_derive::Parser;

use crate::blackboard::{Blackboard, EntryRef};

#[derive(Parser)]
#[grammar = "expr.pest"]
pub struct ExprParser;

static PRATT_PARSER: OnceLock<PrattParser<Rule>> = OnceLock::new();

fn pratt_parser_base() -> PrattParser<Rule> {
    use Rule::*;
    use pest::pratt_parser::{Assoc::*, Op};

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
) -> PrattParserMap<'pratt, 'a, 'b, Rule, impl FnMut(Pair<'b, Rule>) -> Expr, Expr> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::integer => Expr::Value(Value::Integer(primary.as_str().parse::<i64>().unwrap())),
            Rule::float => Expr::Value(Value::Float(primary.as_str().parse::<f64>().unwrap())),
            Rule::string => Expr::Value(Value::String(primary.into_inner().find_first_tagged("name").expect("string should have a \"name\"-tagged child").as_str().into())),
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
                value: Box::new(rhs)
            }
        })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string value.
    String(String),
    /// A float value.
    Float(f64),
    /// An integer value.
    Integer(i64),
    /// A boolean value.
    Boolean(bool),
    /// An empty value.
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// A string value.
    String,
    /// A float value.
    Float,
    /// An integer value.
    Integer,
    /// A boolean value.
    Boolean,
    /// An empty value.
    Empty,
}

#[derive(Debug)]
pub enum ValueOrAny {
    /// Expression value
    Value(Value),
    /// Value from the Blackboard
    Any(EntryRef),
}

#[derive(Debug, PartialEq)]
pub enum ValuePointer {
    LocalVariable(String),
    BlackboardPointer(String),
}

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

impl Value {
    pub fn as_type(&self) -> ValueType {
        match self {
            Value::String(_) => ValueType::String,
            Value::Float(_) => ValueType::Float,
            Value::Integer(_) => ValueType::Integer,
            Value::Boolean(_) => ValueType::Boolean,
            Value::Empty => ValueType::Empty,
        }
    }
}

impl ValuePointer {
    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Self::LocalVariable(name) => Some(name),
            _ => None,
        }
    }
}

impl Expr {
    pub fn validate(&self) -> anyhow::Result<()> {
        let mut variables = Vec::new();

        Self::validate_recursive(self, &mut variables)
    }

    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_value_pointer(&self) -> Option<&ValuePointer> {
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
                                return Err(anyhow::format_err!("Negation operator not allowed on strings."))
                            }
                            Expr::Value(Value::Boolean(_)) => {
                                return Err(anyhow::format_err!("Negation operator not allowed on booleans."))
                            }
                            _ => ()
                        }
                    }
                    Op::Not => {
                        // Negation operator not allowed on string or boolean literals
                        match value.deref() {
                            Expr::Value(Value::Boolean(_)) | Expr::Value(Value::Integer(_)) => (),
                            Expr::Value(_) => {
                                return Err(anyhow::format_err!("NOT operator only allowed on booleans and integers."))
                            }
                            _ => ()
                        }
                    }
                    _ => return Err(anyhow::format_err!("Expected unary operator, found {op:?}"))
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

    fn eval_with_context_recursive(expr: &Expr, context: &mut ContextInternal) -> ExprResult<Value> {
        match expr {
            Expr::Value(value) => Ok(value.clone()),
            Expr::ValuePointer(value_pointer) => {
                match value_pointer {
                    ValuePointer::LocalVariable(name) => {
                        context
                            .context
                            .get_value(name)
                            .or_else(|| context.variables.get(name))
                            .cloned()
                            .ok_or_else(|| anyhow::format_err!("Context didn't contain variable"))
                    }
                    ValuePointer::BlackboardPointer(key) => {
                        todo!()
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
            Expr::BinaryOp { lhs, op, rhs } => {
                match op {
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
                            
                            todo!()
                        } else {
                            unreachable!("BlackboardAssign should have BlackboardPointer as LHS")
                        }
                    }
                    op => {
                        let lhs = Self::eval_with_context_recursive(lhs, context)?;
                        let rhs = Self::eval_with_context_recursive(rhs, context)?;
        
                        op.binary(&lhs, &rhs)
                    }
                }
            }
            Expr::UnaryOp { op, value } => {
                let value = Self::eval_with_context_recursive(value, context)?;

                op.unary(&value)
            }
            Expr::FunctionCall { name, args } => {
                let args = args
                    .iter()
                    .map(|arg| {
                        match arg {
                            Expr::ValuePointer(ValuePointer::BlackboardPointer(key)) => {
                                context
                                    .context
                                    .blackboard
                                    .get_entry_ref(key)
                                    .map(ValueOrAny::Any)
                                    .ok_or_else(|| anyhow::format_err!("Blackboard key {key} doesn't exist."))
                            }
                            expr => {
                                Self::eval_with_context_recursive(expr, context)
                                    .map(ValueOrAny::Value)
                            }
                        }
                    })
                    .collect::<ExprResult<Vec<_>>>()?;
                
                context.context.call_function(name, &args)
            }
        }
    }
}

impl Op {
    fn binary(&self, lhs: &Value, rhs: &Value) -> ExprResult<Value> {
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
            name => unreachable!("expected a binary operator, got {name:?}")
        }
    }

    fn unary(&self, value: &Value) -> ExprResult<Value> {
        match self {
            Op::Neg => value.neg(),
            Op::Not => Ok(Value::Boolean(value.not())),
            name => unreachable!("expected a unary operator, got {name:?}"),
        }
    }
}

#[allow(clippy::should_implement_trait)]
impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::String(value) => !value.is_empty(),
            Value::Float(_) => !self.eq(&Value::Float(0.0)),
            Value::Integer(value) => *value != 0,
            Value::Boolean(value) => *value,
            Value::Empty => false,
        }
    }

    pub fn as_int(&self) -> ExprResult<i64> {
        match self {
            Value::Float(value) => Ok(*value as i64),
            Value::Integer(value) => Ok(*value),
            Value::Boolean(value) => if *value { Ok(1) } else { Ok(0) },
            name => Err(anyhow::format_err!("{name:?} cannot be represented as an integer")),
        }
    }

    pub fn as_float(&self) -> ExprResult<f64> {
        match self {
            Value::Float(value) => Ok(*value),
            Value::Integer(value) => Ok(*value as f64),
            Value::Boolean(value) => if *value { Ok(1.0) } else { Ok(0.0) },
            name => Err(anyhow::format_err!("{name:?} cannot be represented as a float")),
        }
    }

    pub fn checked_add(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Addition is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Addition is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value + *other,
                    Value::Integer(other) => *value + (*other as f64),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 + *other),
                    Value::Integer(other) => Value::Integer(value.checked_add(*other).ok_or_else(|| anyhow::format_err!("Integer overflow during addition"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn checked_sub(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Subtraction is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Subtraction is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value - *other,
                    Value::Integer(other) => *value - (*other as f64),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 - *other),
                    Value::Integer(other) => Value::Integer(value.checked_sub(*other).ok_or_else(|| anyhow::format_err!("Integer overflow during subtraction"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn checked_mul(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Multiplication is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Multiplication is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value * *other,
                    Value::Integer(other) => *value * (*other as f64),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 * *other),
                    Value::Integer(other) => Value::Integer(value.checked_mul(*other).ok_or_else(|| anyhow::format_err!("Integer overflow during multiplication"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn checked_div(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Division is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Division is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value / *other,
                    Value::Integer(other) => *value / (*other as f64),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 / *other),
                    Value::Integer(other) => Value::Integer(value.checked_div(*other).ok_or_else(|| anyhow::format_err!("Integer overflow during division"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn checked_mod(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => *value % *other,
                    Value::Integer(other) => *value % (*other as f64),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float(*value as f64 % *other),
                    Value::Integer(other) => Value::Integer(value.checked_rem(*other).ok_or_else(|| anyhow::format_err!("Integer overflow during modulo"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn checked_pow(&self, other: &Value) -> ExprResult<Value> {
        if !matches!(self.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {self:?}"));
        }

        if !matches!(other.as_type(), ValueType::Float | ValueType::Integer) {
            return Err(anyhow::format_err!("Modulo is not allowed for {other:?}"));
        }
        
        match self {
            Value::Float(value) => {
                let value = match other {
                    Value::Float(other) => value.powf(*other),
                    Value::Integer(other) => value.powi(i32::try_from(*other)?),
                    _ => unreachable!()
                };

                Ok(Value::Float(value))
            }
            Value::Integer(value) => {
                let value = match other {
                    Value::Float(other) => Value::Float((*value as f64).powf(*other)),
                    Value::Integer(other) => Value::Integer(value.checked_pow(u32::try_from(*other)?).ok_or_else(|| anyhow::format_err!("Integer overflow during exponentiation"))?),
                    _ => unreachable!()
                };

                Ok(value)
            }
            _ => unreachable!()
        }
    }

    pub fn eq(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value == other
                } else {
                    false
                }
            }
            Value::Float(value) => {
                match other {
                    Value::Integer(other) => f64::abs(*value - *other as f64) < f64::EPSILON,
                    Value::Float(other) => f64::abs(*value - *other) < f64::EPSILON,
                    Value::Boolean(other) => {
                        let bool_val = if *other { 1 } else { 0 };
                        f64::abs(*value - bool_val as f64) < f64::EPSILON
                    }
                    _ => false,
                }
            }
            Value::Integer(value) => {
                match other {
                    Value::Integer(other) => *value == *other,
                    Value::Float(other) => f64::abs(*value as f64 - *other) < f64::EPSILON,
                    Value::Boolean(other) => {
                        let bool_val = if *other { 1 } else { 0 };
                        *value == bool_val
                    }
                    _ => false,
                }
            }
            Value::Boolean(value) => {
                *value == other.is_truthy()
            }
            Value::Empty => matches!(other, Value::Empty),
        }
    }

    pub fn neq(&self, other: &Value) -> bool {
        !self.eq(other)
    }

    pub fn gt(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value > other
                } else {
                    false
                }
            }
            Value::Float(value) => {
                match other {
                    Value::Integer(other_val) => self.neq(other) && *value > (*other_val as f64),
                    Value::Float(other_val) => self.neq(other) && *value > *other_val,
                    Value::Boolean(other_val) => {
                        let bool_val = if *other_val { 1 } else { 0 };
                        self.neq(other) && *value > bool_val as f64
                    }
                    _ => false,
                }
            }
            Value::Integer(value) => {
                match other {
                    Value::Integer(other) => *value > *other,
                    Value::Float(other_val) => self.neq(other) && (*value as f64) > *other_val,
                    Value::Boolean(other_val) => {
                        let bool_val = if *other_val { 1 } else { 0 };
                        self.neq(other) && *value > bool_val
                    }
                    _ => false,
                }
            }
            Value::Boolean(value) => {
                if let Value::Boolean(other) = other {
                    let value = if *value { 1 } else { 0 };
                    let other = if *other { 1 } else { 0 };
                    
                    value > other
                } else {
                    false
                }
            }
            Value::Empty => false,
        }
    }

    pub fn lt(&self, other: &Value) -> bool {
        match self {
            Value::String(value) => {
                if let Value::String(other) = other {
                    value < other
                } else {
                    false
                }
            }
            Value::Float(value) => {
                match other {
                    Value::Integer(other_val) => self.neq(other) && *value < (*other_val as f64),
                    Value::Float(other_val) => self.neq(other) && *value < *other_val,
                    Value::Boolean(other_val) => {
                        let bool_val = if *other_val { 1 } else { 0 };
                        self.neq(other) && *value < bool_val as f64
                    }
                    _ => false,
                }
            }
            Value::Integer(value) => {
                match other {
                    Value::Integer(other) => *value < *other,
                    Value::Float(other_val) => self.neq(other) && (*value as f64) < *other_val,
                    Value::Boolean(other_val) => {
                        let bool_val = if *other_val { 1 } else { 0 };
                        self.neq(other) && *value < bool_val
                    }
                    _ => false,
                }
            }
            Value::Boolean(value) => {
                if let Value::Boolean(other) = other {
                    let value = if *value { 1 } else { 0 };
                    let other = if *other { 1 } else { 0 };
                    
                    value < other
                } else {
                    false
                }
            }
            Value::Empty => false,
        }
    }

    pub fn geq(&self, other: &Value) -> bool {
        self.gt(other) || self.eq(other)
    }

    pub fn leq(&self, other: &Value) -> bool {
        self.lt(other) || self.eq(other)
    }

    pub fn not(&self) -> bool {
        !self.is_truthy()
    }

    pub fn neg(&self) -> ExprResult<Value> {
        match self {
            Value::Float(value) => Ok(Value::Float(-*value)),
            Value::Integer(value) => Ok(Value::Integer(-*value)),
            Value::Boolean(_) => Ok(Value::Integer(-self.as_int()?)),
            name => Err(anyhow::format_err!("Negation cannot be performed on {name:?}"))
        }
    }

    pub fn and(&self, other: &Value) -> bool {
        self.is_truthy() && other.is_truthy()
    }

    pub fn or(&self, other: &Value) -> bool {
        self.is_truthy() || other.is_truthy()
    }
}

pub type ExprResult<T> = anyhow::Result<T>;
type FunctionType = dyn Fn(&[ValueOrAny]) -> ExprResult<Value>;

pub struct Function {
    f: Box<FunctionType>,
}

impl Function {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&[ValueOrAny]) -> ExprResult<Value> + 'static,
    {
        Self {
            f: Box::new(f),
        }
    }

    pub fn call(&self, args: &[ValueOrAny]) -> ExprResult<Value> {
        (self.f)(args)
    }
}

impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function").finish()
    }
}

struct ContextInternal<'a> {
    context: &'a Context,
    variables: HashMap<String, Value>,
}

#[derive(Debug, Default)]
pub struct Context {
    variables: HashMap<String, Value>,
    functions: HashMap<String, Function>,
    blackboard: Blackboard,
}

impl Context {
    pub fn new(blackboard: Blackboard) -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            blackboard,
        }
    }

    pub fn get_value(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn get_value_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut(name)
    }

    pub fn set_value(&mut self, name: String, value: Value) -> ExprResult<()> {
        if let Some(existing) = self.get_value_mut(&name) {
            if existing.as_type() == value.as_type() {
                *existing = value;
            } else {
                return Err(anyhow::format_err!("Can't change the type of a variable"));
            }
        } else {
            self.variables.insert(name, value);
        }

        Ok(())
    }

    pub fn add_function(&mut self, name: impl Into<String>, f: Function) -> ExprResult<()> {
        self.functions.insert(name.into(), f);

        Ok(())
    }

    pub fn call_function(&self, name: &str, args: &[ValueOrAny]) -> ExprResult<Value> {
        self
            .functions
            .get(name)
            .map(|f| f.call(args))
            .ok_or_else(|| anyhow::format_err!("Function {name} not found in context"))?
    }
}

pub fn parse_expr(text: &str) -> anyhow::Result<Expr> {
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

fn lex_expr(text: &str) -> anyhow::Result<Pairs<'_, Rule>> {
    Ok(ExprParser::parse(Rule::root, text)?)
}

fn pairs_to_expr<'a>(pairs: impl Iterator<Item = Pair<'a, Rule>>) -> Expr {
    let parser = PRATT_PARSER.get_or_init(pratt_parser_base);

    pratt_parser(parser).parse(pairs)
}

