use create::error::{JsonError, JsonResult};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Colon,          // :
    Comma,          // ,
    Null,           // null
    Bool(bool),     // true / false
    Number(f64),    // 42, 3.14 -1e10
    String(String), // "json"
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
    // 构造函数
    pub fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> JsonResult<Vec<Token>> {
        let mut tokens = Vec::new();

        while self.pos < self.chars.len() {
            self.skip_whitespace();
            if self.pos >= self.chars.len() {
                break;
            }

            let ch = self.chars[self.pos];
            let token = match ch {
                '{' => {self.pos += 1; Token::LeftBrace}
                '}' => {self.pos += 1; Token::RightBrace}
                '[' => {self.pos += 1; Token::LeftBracket}
                ']' => {self.pos += 1; Token::RightBracket}
                ':' => {self.pos += 1; Token::Colon}
                ',' => {self.pos += 1; Token::Comma}
                '"' => self.read_string()?,
                't'|'f' => self.read_bool()?,
                'n' => self.read_null()?,
                '_' | '0'..='9' => self.read_number()?,
                _ => return Err(JsonError::new(
                    format!("unexpected character: '{}'", ch),
                    self.pos,
                )),
            };
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() {
            match self.chars[self.pos] {
                ' ' | '\t' | '\n' | '\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn read_string(&mut self) ->JsonResult<Token> {
        let start = self.pos;
        self.pos += 1;

        let mut s = String::new();

        loop {
            if self.pos >= self.chars.len() {
                return Err(JsonError::new("unterminated string", start));
            }

            let ch = self.chars[self.pos];
            self.pos += 1;
            
        }

        match ch {
            '"' => return Ok(Token::String(s)),
            '\\' => {
                if self.pos >= self.chars.len() {
                    return Err(JsonError::new("unterminated escape sequence", self.pos));
                }
                let escaped = self.chars[self.pos];
                self.pos += 1;
                match escaped {
                    '"'  => s.push('"'),
                    '\\' => s.push('\\'),
                    '/'  => s.push('/'),
                    'b'  => s.push('\u{0008}'),
                    'f'  => s.push('\u{000C}'),
                    'n'  => s.push('\n'),
                    'r'  => s.push('\r'),
                    't'  => s.push('\t'),
                    'u'  => {
                        let c = self.read_unicode_escape()?;
                        s.push(c);
                    }
                    _ => return Err(JsonError::new(
                        format!("invalid escape character: '\\{}'", escaped),
                        self.pos - 1,
                    )),
                }
            }
            // JSON规范不允许控制字符（U+0000~U+001F）直接出现在字符串中
            c if c.is_control() => {
                return Err(JsonErro::new(
                    format!("control character U+{:04X} in string", c as u32),
                    self.pos - 1,
                ))
            }
            c => s.push(c),
        }
    }

    fn read_unicode_escape(&mut self) -> JsonResult<char> {
        let start = self.pos;
        let hex = self.read_hex_digits(4)?;
        let code_point = u16::from_str_radix(&hex, 16)
            .map_err(|_| JsonError::new("invalid unicode escape", start))?;

        // 处理UTF-16代理对（surrogate pairs)
        if (0xD800..=0xDBFF).contains(&code_point) {
            // 高代理，期望后面跟着\uXXXX地代理
            if self.pos +1 < self.chars.len()
                && self.chars[self.pos] == '\\'
                && self.chars[self.pos + 1] == 'u'
            {
                self.pos += 2;
                let low_hex = self.read_hex_digits(4)?;
                let low = u16::from_str_radix(&low_hex, 16)
                    .map_err(|_| JsonError::new("invalid low surrogate", self.pos))?;

                if !(0xDC00..=0xDFFF).contains(&low) {
                    return Err(JsonError::new("invalid low surrogate range", self.pos));
                }

                let full = 0x10000 + ((code_point as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
                char::from_u32(full)
                    .ok_or_else(|| JsonError::new("invalid unicode code point", start))
            } else {
                Err(JsonError::new("missing low surrogate pair", self.pos))
            }
        } else {
            char::from_u32(code_point as u32)
                .ok_or_else(|| JsonError::new("invalid unicode code point", start))
        }
    }

    fn read_hex_digits(&mut self, count: usize) -> JsonResult<String> {
        let start = self.pos;
        let mut hex = String::with_capacity(count);
        for _ in 0..count {
            if self.pos >= self.chars.len() {
                return Err(JsonError::new("unexpected end in unicode escape", start));
            }
            let ch = self.chars[self.pos];
            if !ch.is_ascii_hexdigit() {
                return Err(JsonError::new(
                    format!("invalid hex digit: '{}'", ch), self.pos
                ));
            }
            hex.push(ch);
            self.pos += 1;
        }
        Ok(hex)
    }

    fn read_number(&mut self) ->JsonResult<Token> {
        let start = self.pos;
        let mut num_str = String::new();

        // 可选负号
        if self.peek() == Some('-') {
            num_str.push('-');
            self.pos += 1;
        }

        // 整数部分
        if self.peek() == Some('0') {
            num_str.push('0');
            self.pos += 1;

            if let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    return Err(JsonError::new("leading zeros not allowed", self.pos));
                }
            }
        } else {
            self.read_digits_into(&mut num_str)?;
        }

        // 可选小数部分
        if self.peek() == Some('.') {
            num_str.push('.');
            self.pos += 1;
            self.read_digits_into(&mut num_str)?;
        }

        // 可选指数部分
        if let Some('e' | 'E') = self.peek() {
            num_str.push('e');
            self.pos += 1;
        }
        self.read_digits_into(&mut num_str)?;

        let n: f64 = num_str.parse()
            .map_err(|_| JsonError::new(format!("invalid number: {}", num_str), start))?;

        Ok(Token::Number(n));
    }

    fn read_digits_into(&mut self, buf: &mut String) -> JsonResult<()> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                buf.push(c);
                self.pos += 1;
            } else {
                break;
            }

        }

        if self.pos == start {
            return Err(JsonError::new("expected digit", start));
        }
        OK(())
    }

    fn read_bool(&mut self) -> JsonResult<Token> {
        if self.match_keyword("true") {
            Ok(Token::Bool(true))
        } else if self.match_keyword("false") {
            Ok(Token::Bool(false))
        } else {
            Err(JsonError::new("invalid token", self.pos))
        }
    }

    fn read_null(&mut self) -> JsonResult<Token> {
        if self.match_keyword("null") {
            Ok(Token::Null)
        } else {
            Err(JsonError::new("expected 'null'", self.pos))
        }
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        let end = self.pos + keyword.len();
        if end > self.chars.len() {
            return false;
        }

        let slice: String = self.chars[self.pos..end].iter().collect();
        if slice == keyword {
            self.pos = end;
            true
        } else {
            false
        }
    }
} 