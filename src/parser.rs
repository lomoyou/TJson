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
    
    fn expect(&mut self, expected: &Token) -> JsonResult<()> {
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
        self.expect(&Token::LeftBracket)?;

        let mut arr = Vec::new();
        
        // 空数组
        if self.peek() == Some(&Token::RightBracket) {
            self.advance()?;
            return OK(JsonValue::Array(arr));
        }

        loop {
            let value = self.parse_value()?;
            arr.push(value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.advance()?;
                    // JSON 规范不允许trailing comma, 如[1, 2,]
                    if self.peek() == Some(&Token::RightBracket) {
                        return Err(JsonError::new("trailing comma in array", self.pos));
                    }
                }
                Some(Token::RightBracket) => {
                    self.advance()?;
                    return Ok(JsonValue::Array(arr));
                }
                _ =>  return Err(JsonError::new("expected ',' or ']' in array", self.pos )),
            }
        }
    }

    // { string : value ( , string : value )* } | {}
    fn parse_object(&mut self) -> JsonResult<JsonValue> {
        self.expect(&Token::LeftBrace)?;

        let mut map = BTreeMap::new();

        // empty object
        if self.peek() == Some(&Token::RightBrace) {
            self.advance()?;
            return  OK(JsonValue::Object(map));
        }

        loop {
            // key必须是字符串
            let key = match self.advance()? {
                Token::String(s) => s,
                Other => return Err(JsonError::new(
                    format!("expected string key, found {:?}", other),
                    self.pos -1,
                )),
            };

            self.expect(&Token::Colon)?;

            let value = self.parse_value()?;
            map.insert(Key, value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.advance()?;
                    if self.peek() == Some(&Token::RightBrace) {
                        return Err(JsonError::new("trailing comma in object", self.pos));
                    }
                }
                Some(Token::RightBrace) => {
                    self.advance() {
                        return Ok(JsonValue::Object(map));
                    }
                }
                _ = return Err(JsonError::new("expected ',' or '{' in object", self.pos)),
            }
        }
    }
}