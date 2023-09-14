use mlua::{Error::ExternalError, Lua, Result, IntoLua, Value as LuaValue};
use pest::iterators::Pair;
use pest::Parser;
use std::char;
use std::collections::HashMap;
use std::sync::Arc;

use crate::val::Value;

#[derive(pest_derive::Parser)]
#[grammar = "json5.pest"]
struct Json5Parser;

// TODO(Joakker): Make this return a Result<String> instead of a naked String.
fn parse_str(pair: Pair<Rule>) -> String {
    let mut s = String::new();
    for c in pair.into_inner() {
        match c.as_rule() {
            Rule::char_literal => s.push_str(c.as_str()),
            Rule::nul_escape_sequence => s.push_str("\u{0000}"),
            Rule::char_escape_sequence => s.push_str(match c.as_str() {
                "n" => "\n",
                "r" => "\r",
                "t" => "\t",
                "b" => "\u{0008}",
                "v" => "\u{000B}",
                "f" => "\u{000C}",
                k => k,
            }),
            Rule::hex_escape_sequence => {
                let hex = match c.as_str().parse() {
                    Ok(n) => n,
                    Err(_) => 0,
                };
                if let Some(c) = char::from_u32(hex) {
                    s.push(c)
                }
            }
            Rule::unicode_escape_sequence => todo!(),
            _ => unreachable!(),
        }
    }
    s
}

fn parse_pair(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::array => Value::Array(pair.into_inner().map(parse_pair).collect()),
        Rule::null => Value::Null,
        Rule::string => Value::String(parse_str(pair)),
        Rule::number => Value::Number(pair.as_str().parse().unwrap()),
        Rule::boolean => Value::Boolean(pair.as_str().parse().unwrap()),
        Rule::object => {
            let pairs = pair.into_inner().map(|pair| {
                let mut inner_rule = pair.into_inner();
                let name = {
                    let pair = inner_rule.next().unwrap();
                    match pair.as_rule() {
                        Rule::identifier => pair.as_str().to_string(),
                        Rule::string => parse_str(pair),
                        _ => unreachable!(),
                    }
                };
                let value = parse_pair(inner_rule.next().unwrap());
                (name, value)
            });
            let mut m = HashMap::new();
            for (k, v) in pairs {
                m.insert(k, v);
            }
            Value::Object(m)
        }
        _ => unreachable!(),
    }
}

pub fn parse<'lua>(lua: &'lua Lua, data: String) -> Result<LuaValue<'lua>> {
    let data = match Json5Parser::parse(Rule::text, data.as_str()) {
        Ok(mut data) => data.next().unwrap(),
        Err(err) => return Err(ExternalError(Arc::new(err))),
    };
    Ok(parse_pair(data).into_lua(lua)?)
}
