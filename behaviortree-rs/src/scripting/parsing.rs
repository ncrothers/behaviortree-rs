use std::{cell::RefCell, str::FromStr};

use winnow::{
    ascii::{dec_int, float},
    combinator::{alt, separated, trace},
    error::{ContextError, ErrMode},
    stream::{AsChar, Stream},
    token::{one_of, take_while},
    ModalResult, Parser,
};

use crate::scripting::value::Value;

use super::{operator::Operator, types::Node};

pub(super) struct ParsingState<'s> {
    variables: RefCell<Vec<&'s str>>,
}

impl ParsingState<'_> {
    pub fn new() -> Self {
        Self {
            variables: RefCell::new(Vec::new()),
        }
    }
}

/// Consumes whitespace only, including line endings
fn whitespace_all<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    take_while(0.., (AsChar::is_newline, AsChar::is_space)).parse_next(input)
}

fn ident<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    fn is_valid(input: char) -> bool {
        input.is_ascii_alphanumeric() || input == '_'
    }

    trace("ident", |input: &mut _| {
        whitespace_all(input)?;
        let start = input.checkpoint();
        // Make sure the first character is a valid ident start
        one_of(('a'..='z', 'A'..='Z', '_')).parse_next(input)?;
        // If it is, now parse the entire thing
        input.reset(&start);
        let ident = take_while(1.., is_valid).parse_next(input)?;

        Ok(ident)
    })
    .parse_next(input)
}

fn parse_to_scalar<T>(input: &mut &str) -> ModalResult<T>
where
    T: FromStr,
{
    take_while(1.., |c: char| {
        !(AsChar::is_space(c)
            || (c.is_ascii_punctuation() && c != '_' && c != '.' && c != '-' && c != '+'))
            || (c == 'e' || c == 'E')
    })
    .parse_to()
    .parse_next(input)
}

pub(super) fn variable_assignment<'a, 's: 'a>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
    |input: &mut _| {
        trace("variable_assignment", |input: &mut _| {
            let var = ident(input)?;

            whitespace_all(input)?;

            "=".parse_next(input)?;

            whitespace_all(input)?;

            todo!()
        })
        .parse_next(input)
    }
}

pub(super) fn expression<'a, 's>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
    move |input: &mut _| alt((literal, literal)).parse_next(input)
}

pub(super) fn literal(input: &mut &str) -> ModalResult<Node> {
    trace("literal", |input: &mut _| {
        // Parse either a float or int
        let value = alt((
            parse_to_scalar::<i64>.map(Value::from),
            parse_to_scalar::<f64>.map(Value::from),
        ))
        .parse_next(input)?;

        Ok(Node {
            operator: Operator::Const { value },
            children: Vec::new(),
        })
    })
    .parse_next(input)
}

pub(super) fn full_expression<'a, 's>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
    move |input: &mut _| {
        let mut statements: Vec<Node> = separated(1.., expression(state), ";").parse_next(input)?;

        // If more than 1 statement is parsed, means it's a chain
        if statements.len() > 1 {
            Ok(Node {
                operator: Operator::RootNode,
                children: vec![Node {
                    operator: Operator::Chain,
                    children: statements,
                }],
            })
        } else {
            Ok(Node {
                operator: Operator::RootNode,
                children: vec![statements.pop().unwrap()],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::int("3", Node {
        operator: Operator::RootNode,
        children: vec![Node {
            operator: Operator::Const {
                value: Value::Int(3)
            },
            children: Vec::new()
        }]
    })]
    #[case::float("123.045", Node {
        operator: Operator::RootNode,
        children: vec![Node {
            operator: Operator::Const {
                value: Value::Float(123.045)
            },
            children: Vec::new()
        }]
    })]
    #[case::float("1e-2", Node {
        operator: Operator::RootNode,
        children: vec![Node {
            operator: Operator::Const {
                value: Value::Float(0.01)
            },
            children: Vec::new()
        }]
    })]
    fn literal(#[case] input: &'static str, #[case] output: Node) {
        let state = ParsingState::new();

        let res = full_expression(&state).parse(input);

        assert_eq!(res, Ok(output));
    }
}
