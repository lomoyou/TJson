mod value;
mod error;
mod lexer;
mod parser;
mod serializer;

pub use error::{JsonError, JsonResult};

pub use crate::value::JsonValue;

pub fn parse(input: &str) -> JsonResult<JsonValue> {
    let mut lexer = lexer::Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

pub fn stringify(value: &JsonValue) -> String {
    serializer::stringify(value)
}

pub fn stringify_pretty(value: &JsonValue, indent: usize) -> String {
    serializer::stringify_pretty(value, indent)
}

#[cfg(test)]
mod tests {
    use super::{parse, JsonValue};

    #[test]
    fn parses_decimal_inside_object() {
        let input = r#"{"version":1.0,"stable":true}"#;
        let value = parse(input).expect("decimal number in object should parse");

        assert_eq!(value["version"], JsonValue::Number(1.0));
        assert_eq!(value["stable"], JsonValue::Bool(true));
    }

    #[test]
    fn parses_exponent_number() {
        let input = r#"{"value":-1.25e+3}"#;
        let value = parse(input).expect("exponent number should parse");

        assert_eq!(value["value"], JsonValue::Number(-1250.0));
    }
}