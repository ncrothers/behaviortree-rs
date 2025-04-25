use std::collections::HashMap;

use crate::Blackboard;

use super::{expr::Function, ExprResult, Value, ValueOrAny};

pub(crate) struct ContextInternal<'a> {
    pub(crate) context: &'a Context,
    pub(crate) variables: HashMap<String, Value>,
}

#[derive(Debug, Default)]
pub struct Context {
    pub(crate) variables: HashMap<String, Value>,
    pub(crate) functions: HashMap<String, Function>,
    pub(crate) blackboard: Blackboard,
}

impl Context {
    /// Create a new, empty `Context` with the provided [`Blackboard`].
    pub fn new(blackboard: Blackboard) -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            blackboard,
        }
    }

    /// Get a reference to the variable value of `name`.
    pub fn get_value(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Get a mutable reference to the variable value of `name`.
    pub fn get_value_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut(name)
    }

    /// Set the value of a variable `name`. Note that variables have static types,
    /// so you cannot change the type of a variable after it has been set.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    /// use behaviortree_rs::scripting::Context;
    ///
    /// let blackboard = Blackboard::new();
    ///
    /// let mut context = Context::new(blackboard);
    ///
    /// let res = context.set_value("foo", 123i64);
    /// assert!(res.is_ok());
    ///
    /// // You cannot change the type of "foo", so this will return an error
    /// let res = context.set_value("foo", true);
    /// assert!(res.is_err());
    /// ```
    pub fn set_value(
        &mut self,
        name: impl Into<String>,
        value: impl Into<Value>,
    ) -> ExprResult<()> {
        let name = name.into();
        let value = value.into();

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

    /// Add a function to be made available in the context, allowing expressions to call
    /// this function.
    ///
    /// Function definitions must implement `Fn(Vec<ValueOrAny>) -> ExprResult<Value>`.
    /// See [`ValueOrAny`] for how to handle the incoming arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use behaviortree_rs::prelude::*;
    ///
    /// fn custom_function(args: Vec<ValueOrAny>) -> ExprResult<Value> {
    ///     // ...
    ///     Ok(Value::Empty)
    /// }
    ///
    /// let blackboard = Blackboard::new();
    /// let mut context = Context::new(blackboard);
    ///
    /// context.add_function("custom_function", custom_function);
    ///
    /// let res = Expr::parse("custom_function(10)");
    /// assert!(res.is_ok());
    ///
    /// let expr = res.unwrap();
    /// let res = expr.eval_with_context(&context);
    ///
    /// assert!(matches!(res, Ok(Value::Empty)));
    /// ```
    pub fn add_function(&mut self, name: impl Into<String>, f: impl Into<Function>) {
        self.functions.insert(name.into(), f.into());
    }

    pub(crate) fn call_function(&self, name: &str, args: Vec<ValueOrAny>) -> ExprResult<Value> {
        self.functions
            .get(name)
            .map(|f| f.call(args))
            .ok_or_else(|| anyhow::format_err!("Function {name} not found in context"))?
    }
}
