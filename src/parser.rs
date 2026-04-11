use crate::error::{JsonError, JsonResult};
use crate::Lexer::Token;
use crate::value::JsonValue;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) ->Self{
        Parser {tokens, pos: 0}
    }

    pub fn parse(&mut self) -> JsonResult<JsonValue> {
        let value = self.parse_value()?;

        if self.pos < self.tokens.len() {
            return Err(JsonError::new("unexpected token after json value", self.pos));
        }
        OK(value)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) ->JsonResult<Token> {
        if self.pos >= self.tokens.len() {
            return Err(JsonError::new("unexpected end of input", self.pos));
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        OK(token)
    }
    
    fn expecte&mut self, expected: &Token) -> JsonResult<()> {
        let token = self.advance()?;
        if &token != expected {
            return Err(JsonError::new(
                format!("expected {:?}", expected, token),
                self.pos - 1,
            ));
        }
        Ok(())
    }

    fn parse_value(&mut self) -> JsonResult<JsonValue> {
        match self.peek() {
            Some(Token::Null) => {
                self.advance()?;
                Ok(JsonValue::Null)
            }
            Some(Token::Bool(_)) => {
                if let Tokem::Bool(b) = self.advance()? {
                    Ok(JsonValue::Bool(b))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Number(_)) => {
                if let Tokem::Number(n) = self.advance()? {
                    Ok(JsonValue::Number(n))
                } else {
                    unreachable!()
                }
            }
            Some(Token::String(_)) => {
                if let Tokem::String(s) = self.advance()? {
                    Ok(JsonValue::String(s))
                } else {
                    unreachable!()
                }
            }
            Some(Token::LeftBracket) => self.parse_array(),
            Some(Token::LeftBrace) => self.parse_object(),
            Some(other) => Err(JsonError::new(
                format!("unexpected token: {:?}", other),
                self.pos
            )),
            None => Err(JsonError::new("unexpected end of input", self.pos)),
        }
    }

    // [ value ( , value )* ] | []
    fn parse_array(&mut self) -> JsonResult<JsonValue> {
        
    }
}