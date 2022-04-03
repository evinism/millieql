use pest::iterators::Pair;
use std::str::FromStr;

use crate::eval::{expr, Value};
use crate::{Error, Result, Rule};

mod count;
mod float;
mod keys;
mod log;
mod map;
mod string;
mod sum;
mod values;

enum Function {
    Count,
    Float,
    Index,
    Keys,
    Log,
    Map,
    String,
    Sum,
    Values,
}

impl FromStr for Function {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "count" => Ok(Function::Count),
            "float" => Ok(Function::Float),
            "index" => Ok(Function::Index),
            "keys" => Ok(Function::Keys),
            "log" => Ok(Function::Log),
            "map" => Ok(Function::Map),
            "string" => Ok(Function::String),
            "sum" => Ok(Function::Sum),
            "values" => Ok(Function::Values),
            _ => Err(Error::query(format!("unknown function {}", s))),
        }
    }
}

impl Function {
    fn takes_fn_arg(&self) -> bool {
        match self {
            Function::Map => true,
            _ => false,
        }
    }
}

pub fn eval(pair: Pair<Rule>, data: &Value, context: Option<Value>) -> Result<Value> {
    let mut function_iter = pair.clone().into_inner();
    let function: Function = match pair.as_rule() {
        Rule::ident => pair.as_str().parse()?,
        Rule::function => function_iter.next().unwrap().as_str().parse()?,
        _ => unreachable!(),
    };

    let mut use_implicit_context = true;
    let fn_arg = match function.takes_fn_arg() {
        true => Some(function_iter.next().unwrap()),
        false => None,
    };
    let mut args = vec![];

    // lot of mutation required here but we need to capture use_implicit_context
    // as a side effect so an iterator chain is even more gross
    for arg in function_iter {
        match arg.as_rule() {
            Rule::at => {
                args.push(expr::eval(arg, data, context.clone())?);
                use_implicit_context = false
            }
            _ => args.push(expr::eval(arg, data, None)?),
        }
    }

    // if context is present, it becomes the last arg
    if use_implicit_context {
        if let Some(ctx) = context {
            args.push(ctx);
        }
    }

    match function {
        Function::Count => count::count(args),
        Function::Float => float::float(args),
        Function::Index => super::index::index(args),
        Function::Keys => keys::keys(args),
        Function::Log => log::log(args),
        Function::Map => map::map(fn_arg.unwrap(), args),
        Function::String => string::string(args),
        Function::Sum => sum::sum(args),
        Function::Values => values::values(args),
    }
}

#[cfg(test)]
mod tests {
    use crate::{MistQLParser, Rule};

    #[test]
    fn parses_basic_function_call() {
        parses_to! {
            parser: MistQLParser,
            input: "count [1,2,3]",
            rule: Rule::query,
            tokens: [
                function(0,13, [
                    ident(0,5),
                    array(6,13, [
                        number(7,8),
                        number(9,10),
                        number(11,12)
                    ])
                ])
            ]
        }
    }

    #[test]
    fn parses_function_with_three_arguments() {
        parses_to! {
            parser: MistQLParser,
            input: "if false 1 2",
            rule: Rule::query,
            tokens: [
                function(0,12, [
                    ident(0,2),
                    bool(3,8),
                    number(9,10),
                    number(11,12)
                ])
            ]
        }
    }

    #[test]
    fn functions_are_first_class_citizens() {
        parses_to! {
            parser: MistQLParser,
            input: "(if toggle keys values) {one: \"two\"}",
            rule: Rule::query,
            tokens: [
                function(0,36, [
                    function(1,22, [
                        ident(1,3),
                        ident(4,10),
                        ident(11,15),
                        ident(16,22)
                    ]),
                    object(24,36, [
                        keyval(25,35, [
                            ident(25,28),
                            string(30,35, [
                                inner(31,34)
                            ])
                        ])
                    ])
                ])
            ]
        }
    }

    #[test]
    fn function_with_function_as_parameter() {
        parses_to! {
            parser: MistQLParser,
            input: "reduce @[0] + @[1] 0 @",
            rule: Rule::query,
            tokens: [
                function(0,22, [
                    ident(0,6),
                    infix_expr(7,19, [
                        indexed_value(7,11, [
                            at(7,8),
                            number(9,10)
                        ]),
                        plus_op(12,13),
                        indexed_value(14,18, [
                            at(14,15),
                            number(16,17)
                        ])
                    ]),
                    number(19,20),
                    at(21,22)
                ])
            ]
        }
    }
}
