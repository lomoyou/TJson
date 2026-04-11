use std::collections::HashMap;

// Json值的类型
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

pub struct JsonParser {
    input: Vec<char>,
    index: usize,
}

impl JosnParser {
    pub fn new(input: &str) -> Self {
        JsonParser {
            input: input.chars().collect(),
            index: 0,
        }
    }
}
