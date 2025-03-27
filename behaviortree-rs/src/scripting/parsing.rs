use std::{cell::RefCell, str::FromStr};

use winnow::{
    ascii::{dec_int, float, multispace0},
    combinator::{alt, cut_err, not, peek, separated, terminated, trace},
    error::{ContextError, ErrMode},
    stream::{AsChar, Stream},
    token::{one_of, take_till, take_until, take_while},
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

fn variable_name<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    trace("variable_name", |input: &mut _| {
        let name = ident(input)?;
        // Make sure it's not a function call
        peek(not('(')).parse_next(input)?;

        Ok(name)
    })
    .parse_next(input)
}

fn bb_pointer<'s>(input: &mut &'s str) -> ModalResult<&'s str> {
    "{".parse_next(input)?;

    let inner_ident = cut_err(ident).parse_next(input)?;

    cut_err("}").parse_next(input)?;

    Ok(inner_ident)
}

fn unit_operand<'a, 's: 'a>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
    |input: &mut _| {
        trace("unit_operand", |input: &mut _| {
            let operator = alt((
                ident.map(|name| Operator::VariableIdentifierRead { identifier: name.into() }),
                bb_pointer.map(|name| Operator::BlackboardKeyIdentifierRead { identifier: name.into() }),
                literal_op,
            )).parse_next(input)?;

            Ok(Node::empty(operator))
        })
        .parse_next(input)
    }
}

fn function_args<'a, 's: 'a>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Vec<Node>, ErrMode<ContextError>> + 'a {
    |input: &mut _| {
        trace("function_args", |input: &mut _| {
            whitespace_all(input)?;
            
            let args: Vec<Vec<Node>> = separated(0.., expression(state), ',').parse_next(input)?;

            Ok(args.into_iter().map(|expr| Node::new(Operator::RootNode, expr)).collect())
        })
        .parse_next(input)
    }
}

fn function_name<'a, 's: 'a>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, &'s str, ErrMode<ContextError>> + 'a {
    |input: &mut _| {
        trace("function", |input: &mut _| {
            whitespace_all(input)?;
            
            terminated(ident, '(').parse_next(input)
        })
        .parse_next(input)
    }
}

fn operator<'a, 's: 'a>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Operator, ErrMode<ContextError>> + 'a {
    |input: &mut _| {
        trace("operator", |input: &mut _| {
            alt((
                // Split alt because tuple was too long
                alt((
                    "==".map(|_| Operator::Eq),
                    "!=".map(|_| Operator::Neq),
                    ">".map(|_| Operator::Gt),
                    ">=".map(|_| Operator::Geq),
                    "<".map(|_| Operator::Lt),
                    "<=".map(|_| Operator::Leq),
                    "&&".map(|_| Operator::And),
                    "||".map(|_| Operator::Or),
                    "!".map(|_| Operator::Not),
                    "=".map(|_| Operator::Assign),
                    ":=".map(|_| Operator::Walrus),
                )),
                alt((
                    '+'.map(|_| Operator::Add),
                    '-'.map(|_| Operator::Sub),
                    '*'.map(|_| Operator::Mul),
                    '/'.map(|_| Operator::Div),
                    '%'.map(|_| Operator::Mod),
                    '-'.map(|_| Operator::Neg), // TODO
                    '^'.map(|_| Operator::Exp),
                ))
            )).parse_next(input)
        })
        .parse_next(input)
    }
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

            // TODO
            todo!()
        })
        .parse_next(input)
    }
}

// pub(super) fn addition<'a, 's: 'a>(
//     state: &'a ParsingState<'s>,
// ) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
//     |input: &mut _| {
//         trace("variable_assignment", |input: &mut _| {
//             let mut left = take_until(1.., '+').parse_next(input)?;
//             let left = cut_err(expression(state)).parse_next(&mut left)?;

//             "+".parse_next(input)?;

//             let right = cut_err(expression(state)).parse_next(input)?;

//             Ok(Node {
//                 operator: Operator::Add,
//                 children: vec![left, right],
//             })
//         })
//         .parse_next(input)
//     }
// }

pub(super) fn expression<'a, 's>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Vec<Node>, ErrMode<ContextError>> + 'a {
    move |input: &mut &'s str| {
        let mut children = Vec::new();

        while !input.is_empty() {
            multispace0.parse_next(input)?;

            let item = alt((
                parentheses_wrapped_expression(state),
                variable_name.map(|name| Node::empty(Operator::VariableIdentifierRead { identifier: name.into() })),
                function_name(state).map(|name| Node::empty(Operator::FunctionIdentifier { identifier: name.into() })),
                operator(state).map(Node::empty),
                literal,
                ';'.map(|_| Node::empty(Operator::Chain)),
            )).parse_next(input)?;

            if matches!(item.operator, Operator::FunctionIdentifier { .. }) {
                let args = cut_err(function_args(state)).parse_next(input)?;
                
                children.push(item);
                children.push(Node::new(Operator::RootNode, args));
            } else {
                children.push(item);
            }
        }

        Ok(children)
    }
}

pub(super) fn parentheses_wrapped_expression<'a, 's>(
    state: &'a ParsingState<'s>,
) -> impl Parser<&'s str, Node, ErrMode<ContextError>> + 'a {
    move |input: &mut _| {
        // Check for a leading open parenthesis
        "(".parse_next(input)?;
        // If we match a "(", if we don't find a closing ")" then we need to cut
        let mut inner = cut_err(take_till(1.., ')')).parse_next(input)?;

        cut_err(")").parse_next(input)?;

        // Parse the inner expression
        // If this inner expression is invalid, need to cut
        let inner = cut_err(expression(state)).parse_next(&mut inner)?;

        Ok(Node::new(Operator::RootNode, inner))
    }
}

pub(super) fn literal(input: &mut &str) -> ModalResult<Node> {
    let operator = literal_op.parse_next(input)?;

    Ok(Node {
        operator,
        children: Vec::new(),
    })
}

pub(super) fn literal_op(input: &mut &str) -> ModalResult<Operator> {
    trace("literal", |input: &mut _| {
        // Parse either a float or int
        let value = alt((
            parse_to_scalar::<i64>.map(Value::from),
            parse_to_scalar::<f64>.map(Value::from),
        ))
        .parse_next(input)?;

        Ok(Operator::Const { value })
    })
    .parse_next(input)
}

pub(super) fn parse_expression<'a, 's>(
    state: &'a ParsingState<'s>,
    expr: &'s str,
) -> anyhow::Result<Node> {
    let mut input = expr;
    let mut statements: Vec<Node> = expression(state).parse(&mut input).map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    if let Some(end) = statements.iter().next_back() {
        // Return an error if the expression doesn't end by returning a value
        if matches!(end.operator, Operator::Chain) {
            return Err(anyhow::format_err!("Expressions must return a value."));
        }
    }

    Ok(Node::new(Operator::RootNode, statements))
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

        let res = parse_expression(&state, input);

        assert!(matches!(res, Ok(output)));
    }

    #[rstest]
    #[case::one_float("(1.0)", Node {
        operator: Operator::RootNode,
        children: vec![Node {
            operator: Operator::RootNode,
            children: vec![Node {
                operator: Operator::Const {
                    value: Value::Float(1.0)
                },
                children: Vec::new()
            }]
        }]
    })]
    fn parentheses(#[case] input: &'static str, #[case] output: Node) {
        let state = ParsingState::new();

        let res = parse_expression(&state, input);

        if let Err(e) = res.as_ref() {
            println!("{e}");
        }

        assert!(matches!(res, Ok(output)));
    }
}
