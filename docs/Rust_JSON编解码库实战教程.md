# Rust JSON 编解码库实战教程

> 从零开始用 Rust 实现一个完整的 JSON 编解码库，覆盖 Lexer → Parser → Serializer → 宏

---

## 目录

- [第一章 架构设计](#第一章-架构设计)
- [第二章 核心类型定义](#第二章-核心类型定义)
- [第三章 Lexer 词法分析器](#第三章-lexer-词法分析器)
- [第四章 Parser 语法分析器](#第四章-parser-语法分析器)
- [第五章 Serializer 序列化器](#第五章-serializer-序列化器)
- [第六章 API 封装与使用示例](#第六章-api-封装与使用示例)
- [第七章 实现 json!() 过程宏](#第七章-实现-json-过程宏)
- [第八章 测试用例](#第八章-测试用例)
- [第九章 进阶方向](#第九章-进阶方向)

---

## 第一章 架构设计

### 1.1 JSON 规范速览（RFC 8259）

JSON 只有 6 种数据类型：

| 类型 | 示例 | Rust 映射 |
|------|------|-----------|
| Null | `null` | 枚举变体 `Null` |
| Boolean | `true` / `false` | `bool` |
| Number | `42`, `3.14`, `-1e10` | `f64` |
| String | `"hello"` | `String` |
| Array | `[1, "a", null]` | `Vec<JsonValue>` |
| Object | `{"k": "v"}` | `BTreeMap<String, JsonValue>` |

**设计决策**：Number 统一用 `f64` 而不区分整数/浮点。这是 JSON 规范本身不区分 int/float 的自然映射。实际生产库（如 serde_json）会用一个内部枚举来区分，但对于学习来说 `f64` 足够。

**设计决策**：Object 用 `BTreeMap` 而非 `HashMap`，好处是序列化输出的 key 有序，方便测试和对比。

### 1.2 整体模块划分

```
json_lib/
├── src/
│   ├── lib.rs          # 公开 API：parse(), stringify()
│   ├── value.rs        # JsonValue 枚举 + 辅助 trait 实现
│   ├── error.rs        # JsonError 错误类型
│   ├── lexer.rs        # 词法分析：&str → Vec<Token>
│   ├── parser.rs       # 语法分析：Vec<Token> → JsonValue
│   └── serializer.rs   # 序列化：JsonValue → String
├── json_macro/         # 过程宏 crate（独立 crate）
│   ├── Cargo.toml
│   └── src/lib.rs
├── Cargo.toml
└── tests/
    └── integration.rs
```

### 1.3 数据流

```
                  Lexer                Parser              Serializer
  "&str" ──────────────► Vec<Token> ──────────► JsonValue ──────────────► String

  输入JSON字符串        词法单元流          语法树(AST)         输出JSON字符串
```

解码（Deserialize）走左半边：`&str → Token → JsonValue`

编码（Serialize）走右半边：`JsonValue → String`

### 1.4 Cargo.toml

```toml
[package]
name = "json_lib"
version = "0.1.0"
edition = "2021"

[dependencies]

[dev-dependencies]
# 无外部依赖，纯 Rust 实现
```

---

## 第二章 核心类型定义

### 2.1 JsonValue — JSON 的 Rust 表示

`src/value.rs`：

```rust
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}
```

为什么 derive 这几个 trait：
- `Debug`：调试打印
- `Clone`：允许值的深拷贝
- `PartialEq`：测试中用 `assert_eq!` 比较

### 2.2 为 JsonValue 实现便捷方法

```rust
impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, JsonValue::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, JsonValue::Number(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, JsonValue::String(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, JsonValue::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, JsonValue::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }
}
```

### 2.3 实现 Index trait — 支持 `value["key"]` 和 `value[0]` 语法

```rust
use std::ops::Index;

// value["key"] 访问 Object 的字段
impl Index<&str> for JsonValue {
    type Output = JsonValue;

    fn index(&self, key: &str) -> &JsonValue {
        match self {
            JsonValue::Object(map) => map.get(key).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}

// value[0] 访问 Array 的元素
impl Index<usize> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: usize) -> &JsonValue {
        match self {
            JsonValue::Array(arr) => arr.get(index).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}
```

**注意**：这里访问不存在的 key/index 返回 `Null` 而非 panic，这样链式访问 `value["a"]["b"][0]` 不会崩溃，行为和 JavaScript 类似。但这是一个有争议的设计——你也可以选择 panic 或返回一个静态 `Null` 引用。

**技巧**：返回 `&JsonValue::Null` 时需要一个具有 `'static` 生命周期的引用。可以用 `const` 来实现：

```rust
const JSON_NULL: JsonValue = JsonValue::Null;

// 然后在 index 中返回 &JSON_NULL
```

但更简洁的方式是直接内联 `&JsonValue::Null`，因为枚举变体没有 Drop，编译器会自动处理。

### 2.4 实现 From trait — 简化构造

```rust
impl From<bool> for JsonValue {
    fn from(b: bool) -> Self { JsonValue::Bool(b) }
}

impl From<f64> for JsonValue {
    fn from(n: f64) -> Self { JsonValue::Number(n) }
}

impl From<i64> for JsonValue {
    fn from(n: i64) -> Self { JsonValue::Number(n as f64) }
}

impl From<&str> for JsonValue {
    fn from(s: &str) -> Self { JsonValue::String(s.to_string()) }
}

impl From<String> for JsonValue {
    fn from(s: String) -> Self { JsonValue::String(s) }
}

impl<T: Into<JsonValue>> From<Vec<T>> for JsonValue {
    fn from(v: Vec<T>) -> Self {
        JsonValue::Array(v.into_iter().map(|x| x.into()).collect())
    }
}
```

这样就能写：

```rust
let v: JsonValue = 42.0.into();
let v: JsonValue = "hello".into();
let v: JsonValue = vec![1.0, 2.0, 3.0].into();
```

### 2.5 Display trait — 直接 println

```rust
impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValue::Null => write!(f, "null"),
            JsonValue::Bool(b) => write!(f, "{}", b),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            JsonValue::String(s) => write!(f, "\"{}\"", s),
            JsonValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 { write!(f, ",")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            JsonValue::Object(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 { write!(f, ",")?; }
                    write!(f, "\"{}\":{}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}
```

### 2.6 JsonError — 统一错误类型

`src/error.rs`：

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonError {
    pub message: String,
    pub position: usize,
}

impl JsonError {
    pub fn new(message: impl Into<String>, position: usize) -> Self {
        JsonError {
            message: message.into(),
            position,
        }
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON error at position {}: {}", self.position, self.message)
    }
}

impl std::error::Error for JsonError {}

pub type JsonResult<T> = Result<T, JsonError>;
```

---

## 第三章 Lexer 词法分析器

Lexer 负责将原始字符串切分为有意义的 **Token**（词法单元）。

### 3.1 Token 类型定义

```rust
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
    Number(f64),    // 42, 3.14, -1e10
    String(String), // "hello"
}
```

### 3.2 Lexer 结构体

```rust
use crate::error::{JsonError, JsonResult};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
}

impl Lexer {
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
                '{' => { self.pos += 1; Token::LeftBrace }
                '}' => { self.pos += 1; Token::RightBrace }
                '[' => { self.pos += 1; Token::LeftBracket }
                ']' => { self.pos += 1; Token::RightBracket }
                ':' => { self.pos += 1; Token::Colon }
                ',' => { self.pos += 1; Token::Comma }
                '"' => self.read_string()?,
                't' | 'f' => self.read_bool()?,
                'n' => self.read_null()?,
                '-' | '0'..='9' => self.read_number()?,
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
}
```

### 3.3 读取字符串 — 处理转义

这是 Lexer 最复杂的部分。JSON 字符串支持以下转义：

| 转义序列 | 含义 |
|----------|------|
| `\"` | 双引号 |
| `\\` | 反斜杠 |
| `\/` | 正斜杠 |
| `\b` | 退格 |
| `\f` | 换页 |
| `\n` | 换行 |
| `\r` | 回车 |
| `\t` | 制表符 |
| `\uXXXX` | Unicode 码点 |

```rust
impl Lexer {
    fn read_string(&mut self) -> JsonResult<Token> {
        let start = self.pos;
        self.pos += 1; // skip opening "

        let mut s = String::new();

        loop {
            if self.pos >= self.chars.len() {
                return Err(JsonError::new("unterminated string", start));
            }

            let ch = self.chars[self.pos];
            self.pos += 1;

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
                // JSON 规范不允许控制字符（U+0000 到 U+001F）直接出现在字符串中
                c if c.is_control() => {
                    return Err(JsonError::new(
                        format!("control character U+{:04X} in string", c as u32),
                        self.pos - 1,
                    ));
                }
                c => s.push(c),
            }
        }
    }

    fn read_unicode_escape(&mut self) -> JsonResult<char> {
        let start = self.pos;
        let hex = self.read_hex_digits(4)?;
        let code_point = u16::from_str_radix(&hex, 16)
            .map_err(|_| JsonError::new("invalid unicode escape", start))?;

        // 处理 UTF-16 代理对 (surrogate pairs)
        if (0xD800..=0xDBFF).contains(&code_point) {
            // 高代理，期望后面跟着 \uXXXX 低代理
            if self.pos + 1 < self.chars.len()
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
}
```

**重点**：`read_unicode_escape` 必须处理 **UTF-16 代理对**（surrogate pairs）。JSON 字符串用 `\uD800\uDC00` 的方式表示 BMP 之外的字符（如 emoji）。这个一定要实现正确，否则 emoji 类的 JSON 会解析失败。

### 3.4 读取数字

JSON 数字的完整格式：`-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?`

```rust
impl Lexer {
    fn read_number(&mut self) -> JsonResult<Token> {
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
            // JSON 不允许 "01"、"007" 这样的前导零
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
            if let Some('+' | '-') = self.peek() {
                num_str.push(self.chars[self.pos]);
                self.pos += 1;
            }
            self.read_digits_into(&mut num_str)?;
        }

        let n: f64 = num_str.parse()
            .map_err(|_| JsonError::new(format!("invalid number: {}", num_str), start))?;

        Ok(Token::Number(n))
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
        Ok(())
    }
}
```

### 3.5 读取 Bool 和 Null

```rust
impl Lexer {
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
```

---

## 第四章 Parser 语法分析器

Parser 将 Token 流转换为 `JsonValue` 树。采用经典的 **递归下降**（Recursive Descent）方法。

### 4.1 Parser 结构体

```rust
use crate::error::{JsonError, JsonResult};
use crate::lexer::Token;
use crate::value::JsonValue;
use std::collections::BTreeMap;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> JsonResult<JsonValue> {
        let value = self.parse_value()?;

        // 整个 JSON 解析完后，不应该还有剩余 token
        if self.pos < self.tokens.len() {
            return Err(JsonError::new("unexpected token after JSON value", self.pos));
        }

        Ok(value)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> JsonResult<Token> {
        if self.pos >= self.tokens.len() {
            return Err(JsonError::new("unexpected end of input", self.pos));
        }
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        Ok(token)
    }

    fn expect(&mut self, expected: &Token) -> JsonResult<()> {
        let token = self.advance()?;
        if &token != expected {
            return Err(JsonError::new(
                format!("expected {:?}, found {:?}", expected, token),
                self.pos - 1,
            ));
        }
        Ok(())
    }
}
```

### 4.2 递归下降 — parse_value

核心分发函数，根据当前 Token 类型决定调用哪个子解析器：

```rust
impl Parser {
    fn parse_value(&mut self) -> JsonResult<JsonValue> {
        match self.peek() {
            Some(Token::Null) => {
                self.advance()?;
                Ok(JsonValue::Null)
            }
            Some(Token::Bool(_)) => {
                if let Token::Bool(b) = self.advance()? {
                    Ok(JsonValue::Bool(b))
                } else {
                    unreachable!()
                }
            }
            Some(Token::Number(_)) => {
                if let Token::Number(n) = self.advance()? {
                    Ok(JsonValue::Number(n))
                } else {
                    unreachable!()
                }
            }
            Some(Token::String(_)) => {
                if let Token::String(s) = self.advance()? {
                    Ok(JsonValue::String(s))
                } else {
                    unreachable!()
                }
            }
            Some(Token::LeftBracket) => self.parse_array(),
            Some(Token::LeftBrace) => self.parse_object(),
            Some(other) => Err(JsonError::new(
                format!("unexpected token: {:?}", other),
                self.pos,
            )),
            None => Err(JsonError::new("unexpected end of input", self.pos)),
        }
    }
}
```

### 4.3 解析 Array

```
语法：[ value ( , value )* ] | []
```

```rust
impl Parser {
    fn parse_array(&mut self) -> JsonResult<JsonValue> {
        self.expect(&Token::LeftBracket)?;

        let mut arr = Vec::new();

        // 空数组
        if self.peek() == Some(&Token::RightBracket) {
            self.advance()?;
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let value = self.parse_value()?;
            arr.push(value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.advance()?;
                    // JSON 规范不允许 trailing comma，如 [1, 2,]
                    if self.peek() == Some(&Token::RightBracket) {
                        return Err(JsonError::new("trailing comma in array", self.pos));
                    }
                }
                Some(Token::RightBracket) => {
                    self.advance()?;
                    return Ok(JsonValue::Array(arr));
                }
                _ => return Err(JsonError::new("expected ',' or ']' in array", self.pos)),
            }
        }
    }
}
```

### 4.4 解析 Object

```
语法：{ string : value ( , string : value )* } | {}
```

```rust
impl Parser {
    fn parse_object(&mut self) -> JsonResult<JsonValue> {
        self.expect(&Token::LeftBrace)?;

        let mut map = BTreeMap::new();

        // 空对象
        if self.peek() == Some(&Token::RightBrace) {
            self.advance()?;
            return Ok(JsonValue::Object(map));
        }

        loop {
            // key 必须是字符串
            let key = match self.advance()? {
                Token::String(s) => s,
                other => return Err(JsonError::new(
                    format!("expected string key, found {:?}", other),
                    self.pos - 1,
                )),
            };

            self.expect(&Token::Colon)?;

            let value = self.parse_value()?;
            map.insert(key, value);

            match self.peek() {
                Some(Token::Comma) => {
                    self.advance()?;
                    if self.peek() == Some(&Token::RightBrace) {
                        return Err(JsonError::new("trailing comma in object", self.pos));
                    }
                }
                Some(Token::RightBrace) => {
                    self.advance()?;
                    return Ok(JsonValue::Object(map));
                }
                _ => return Err(JsonError::new("expected ',' or '}' in object", self.pos)),
            }
        }
    }
}
```

---

## 第五章 Serializer 序列化器

将 `JsonValue` 转回 JSON 字符串。

### 5.1 紧凑输出

`src/serializer.rs`：

```rust
use crate::value::JsonValue;

pub fn stringify(value: &JsonValue) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn write_value(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && n.is_finite() && n.abs() < (i64::MAX as f64) {
                out.push_str(&(*n as i64).to_string());
            } else {
                out.push_str(&n.to_string());
            }
        }
        JsonValue::String(s) => {
            out.push('"');
            write_escaped_string(s, out);
            out.push('"');
        }
        JsonValue::Array(arr) => {
            out.push('[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 { out.push(','); }
                write_value(v, out);
            }
            out.push(']');
        }
        JsonValue::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 { out.push(','); }
                out.push('"');
                write_escaped_string(k, out);
                out.push_str("\":");
                write_value(v, out);
            }
            out.push('}');
        }
    }
}

fn write_escaped_string(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if c.is_control() => {
                // 其他控制字符用 \u00XX 表示
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{:04x}", unit));
                }
            }
            c => out.push(c),
        }
    }
}
```

### 5.2 美化输出（Pretty Print）

```rust
pub fn stringify_pretty(value: &JsonValue, indent_size: usize) -> String {
    let mut output = String::new();
    write_value_pretty(value, &mut output, 0, indent_size);
    output.push('\n');
    output
}

fn write_value_pretty(value: &JsonValue, out: &mut String, depth: usize, indent_size: usize) {
    match value {
        JsonValue::Array(arr) if arr.is_empty() => out.push_str("[]"),
        JsonValue::Object(map) if map.is_empty() => out.push_str("{}"),

        JsonValue::Array(arr) => {
            out.push_str("[\n");
            for (i, v) in arr.iter().enumerate() {
                write_indent(out, depth + 1, indent_size);
                write_value_pretty(v, out, depth + 1, indent_size);
                if i < arr.len() - 1 { out.push(','); }
                out.push('\n');
            }
            write_indent(out, depth, indent_size);
            out.push(']');
        }

        JsonValue::Object(map) => {
            out.push_str("{\n");
            let entries: Vec<_> = map.iter().collect();
            for (i, (k, v)) in entries.iter().enumerate() {
                write_indent(out, depth + 1, indent_size);
                out.push('"');
                write_escaped_string(k, out);
                out.push_str("\": ");
                write_value_pretty(v, out, depth + 1, indent_size);
                if i < entries.len() - 1 { out.push(','); }
                out.push('\n');
            }
            write_indent(out, depth, indent_size);
            out.push('}');
        }

        // 非容器类型直接用紧凑写法
        other => write_value(other, out),
    }
}

fn write_indent(out: &mut String, depth: usize, indent_size: usize) {
    for _ in 0..(depth * indent_size) {
        out.push(' ');
    }
}
```

---

## 第六章 API 封装与使用示例

### 6.1 lib.rs — 公开 API

`src/lib.rs`：

```rust
mod value;
mod error;
mod lexer;
mod parser;
mod serializer;

pub use value::JsonValue;
pub use error::{JsonError, JsonResult};

/// 解析 JSON 字符串为 JsonValue
pub fn parse(input: &str) -> JsonResult<JsonValue> {
    let mut lexer = lexer::Lexer::new(input);
    let tokens = lexer.tokenize()?;
    let mut parser = parser::Parser::new(tokens);
    parser.parse()
}

/// 将 JsonValue 序列化为紧凑 JSON 字符串
pub fn stringify(value: &JsonValue) -> String {
    serializer::stringify(value)
}

/// 将 JsonValue 序列化为美化 JSON 字符串
pub fn stringify_pretty(value: &JsonValue, indent: usize) -> String {
    serializer::stringify_pretty(value, indent)
}
```

### 6.2 使用示例

```rust
use json_lib::{parse, stringify, stringify_pretty, JsonValue};

fn main() {
    // ===== 解码 =====
    let input = r#"{
        "name": "Rust JSON",
        "version": 1.0,
        "features": ["parse", "stringify", "pretty print"],
        "stable": true,
        "metadata": null
    }"#;

    let value = parse(input).expect("parse failed");

    // 用 Index trait 访问
    println!("name: {}", value["name"]);           // "Rust JSON"
    println!("first feature: {}", value["features"][0]); // "parse"
    println!("stable: {}", value["stable"]);        // true
    println!("missing: {}", value["nonexist"]);     // null

    // 用类型方法访问
    if let Some(name) = value["name"].as_str() {
        println!("name is: {}", name);
    }

    // ===== 编码 =====
    let compact = stringify(&value);
    println!("compact: {}", compact);

    let pretty = stringify_pretty(&value, 2);
    println!("pretty:\n{}", pretty);

    // ===== 手动构建 =====
    use std::collections::BTreeMap;

    let mut obj = BTreeMap::new();
    obj.insert("x".to_string(), JsonValue::Number(1.0));
    obj.insert("y".to_string(), JsonValue::Number(2.0));
    let point = JsonValue::Object(obj);

    println!("point: {}", stringify(&point)); // {"x":1,"y":2}
}
```

---

## 第七章 实现 json!() 过程宏

目标：用类似 JSON 字面量的语法在 Rust 中构建 `JsonValue`：

```rust
let value = json!({
    "name": "Rust",
    "version": 1.0,
    "features": ["fast", "safe"],
    "active": true,
    "meta": null
});
```

### 7.1 为什么需要过程宏

Rust 的声明宏 `macro_rules!` 可以实现基本版本，但有一些限制：
- 匹配 JSON 对象的 `"key": value` 语法需要递归展开，很复杂
- 错误信息不友好

声明宏版本适合入门，过程宏版本更强大。这里两种都给出。

### 7.2 声明宏版本（零依赖，在主 crate 中即可）

在 `src/lib.rs` 中添加：

```rust
/// 声明宏版本的 json!()
/// 用法：json!(null), json!(true), json!(42), json!("str"),
///       json!([1, 2, 3]), json!({"key": "value"})
#[macro_export]
macro_rules! json {
    // null
    (null) => {
        $crate::JsonValue::Null
    };

    // bool
    (true) => {
        $crate::JsonValue::Bool(true)
    };
    (false) => {
        $crate::JsonValue::Bool(false)
    };

    // array
    ([ $($element:tt),* $(,)? ]) => {
        $crate::JsonValue::Array(vec![ $( json!($element) ),* ])
    };

    // object
    ({ $( $key:tt : $value:tt ),* $(,)? }) => {
        {
            #[allow(unused_mut)]
            let mut map = ::std::collections::BTreeMap::new();
            $(
                map.insert(String::from($key), json!($value));
            )*
            $crate::JsonValue::Object(map)
        }
    };

    // number (integer literals)
    ($n:tt) => {
        match $n {
            n => {
                // 如果是字符串字面量，会被自动推导
                let v: $crate::JsonValue = n.into();
                v
            }
        }
    };
}
```

**限制说明**：
- 这个声明宏利用了 `tt` (token tree) 匹配，Rust 会根据字面量类型自动选择分支
- 对象的 key 需要是字符串字面量（`"key"`），value 可以嵌套
- 数字字面量默认是 `i32`，需要写 `1.0` 或 `1_f64` 才能匹配 `f64`
  - `From<i64>` trait 可以解决整数到 `f64` 的转换

**使用示例**：

```rust
use json_lib::json;

fn main() {
    let data = json!({
        "name": "test",
        "scores": [95, 87, 92],
        "passed": true,
        "extra": null,
        "nested": {
            "x": 1.0,
            "y": 2.0
        }
    });

    println!("{}", json_lib::stringify_pretty(&data, 2));
}
```

### 7.3 过程宏版本（独立 crate）

过程宏必须在一个单独的 crate 中定义，`proc-macro = true`。

**`json_macro/Cargo.toml`**：

```toml
[package]
name = "json_macro"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
# proc-macro2, syn, quote 是过程宏三件套
proc-macro2 = "1"
syn = { version = "2", features = ["full"] }
quote = "1"
```

**`json_macro/src/lib.rs`**：

```rust
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, Lit, Token, Result as SynResult};

// json!() 的入口
#[proc_macro]
pub fn json(input: TokenStream) -> TokenStream {
    let json_val = syn::parse_macro_input!(input as JsonMacroValue);
    let expanded = json_val.to_tokens();
    expanded.into()
}

// 表示宏输入中的一个 JSON 值
enum JsonMacroValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<JsonMacroValue>),
    Object(Vec<(String, JsonMacroValue)>),
}

impl Parse for JsonMacroValue {
    fn parse(input: ParseStream) -> SynResult<Self> {
        // null / true / false
        if input.peek(syn::Ident) {
            let ident: syn::Ident = input.parse()?;
            return match ident.to_string().as_str() {
                "null" => Ok(JsonMacroValue::Null),
                "true" => Ok(JsonMacroValue::Bool(true)),
                "false" => Ok(JsonMacroValue::Bool(false)),
                other => Err(syn::Error::new(ident.span(), format!("unexpected: {}", other))),
            };
        }

        // 负数 (-42)
        if input.peek(Token![-]) {
            let _: Token![-] = input.parse()?;
            let lit: Lit = input.parse()?;
            if let Lit::Int(i) = &lit {
                let n: f64 = -(i.base10_parse::<i64>()? as f64);
                return Ok(JsonMacroValue::Number(n));
            } else if let Lit::Float(f) = &lit {
                let n: f64 = -f.base10_parse::<f64>()?;
                return Ok(JsonMacroValue::Number(n));
            }
            return Err(syn::Error::new(lit.span(), "expected number after '-'"));
        }

        // 字面量：数字、字符串
        if input.peek(Lit) {
            let lit: Lit = input.parse()?;
            return match lit {
                Lit::Str(s) => Ok(JsonMacroValue::Str(s.value())),
                Lit::Int(i) => Ok(JsonMacroValue::Number(i.base10_parse::<i64>()? as f64)),
                Lit::Float(f) => Ok(JsonMacroValue::Number(f.base10_parse::<f64>()?)),
                _ => Err(syn::Error::new(lit.span(), "unsupported literal")),
            };
        }

        // 数组 [...]
        if input.peek(syn::token::Bracket) {
            let content;
            bracketed!(content in input);
            let mut arr = Vec::new();
            while !content.is_empty() {
                arr.push(content.parse()?);
                if !content.is_empty() {
                    let _: Token![,] = content.parse()?;
                }
            }
            return Ok(JsonMacroValue::Array(arr));
        }

        // 对象 {...}
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let mut pairs = Vec::new();
            while !content.is_empty() {
                let key: syn::LitStr = content.parse()?;
                let _: Token![:] = content.parse()?;
                let value: JsonMacroValue = content.parse()?;
                pairs.push((key.value(), value));
                if !content.is_empty() {
                    let _: Token![,] = content.parse()?;
                }
            }
            return Ok(JsonMacroValue::Object(pairs));
        }

        Err(input.error("expected a JSON value"))
    }
}

impl JsonMacroValue {
    fn to_tokens(&self) -> TokenStream2 {
        match self {
            JsonMacroValue::Null => quote! { json_lib::JsonValue::Null },
            JsonMacroValue::Bool(b) => quote! { json_lib::JsonValue::Bool(#b) },
            JsonMacroValue::Number(n) => quote! { json_lib::JsonValue::Number(#n) },
            JsonMacroValue::Str(s) => quote! { json_lib::JsonValue::String(String::from(#s)) },
            JsonMacroValue::Array(arr) => {
                let items: Vec<_> = arr.iter().map(|v| v.to_tokens()).collect();
                quote! {
                    json_lib::JsonValue::Array(vec![ #(#items),* ])
                }
            }
            JsonMacroValue::Object(pairs) => {
                let inserts: Vec<_> = pairs.iter().map(|(k, v)| {
                    let val = v.to_tokens();
                    quote! { map.insert(String::from(#k), #val); }
                }).collect();
                quote! {
                    {
                        let mut map = ::std::collections::BTreeMap::new();
                        #(#inserts)*
                        json_lib::JsonValue::Object(map)
                    }
                }
            }
        }
    }
}
```

**关键知识点**：

| 概念 | 说明 |
|------|------|
| `proc_macro` crate | Rust 编译器提供的过程宏接口 |
| `syn` | 解析 Rust token stream 为语法树 |
| `quote` | 将语法树重新拼装为 token stream |
| `Parse` trait | 自定义语法解析，类似递归下降 |
| `braced!` / `bracketed!` | syn 提供的匹配括号的便捷宏 |

### 7.4 在主 crate 中使用过程宏

在 `json_lib/Cargo.toml` 中添加依赖：

```toml
[dependencies]
json_macro = { path = "./json_macro" }
```

然后在 `lib.rs` 中 re-export：

```rust
pub use json_macro::json;
```

---

## 第八章 测试用例

`tests/integration.rs`：

```rust
use json_lib::{parse, stringify, stringify_pretty, JsonValue, json};

// ========== 基本类型解析 ==========

#[test]
fn test_parse_null() {
    assert_eq!(parse("null").unwrap(), JsonValue::Null);
}

#[test]
fn test_parse_bool() {
    assert_eq!(parse("true").unwrap(), JsonValue::Bool(true));
    assert_eq!(parse("false").unwrap(), JsonValue::Bool(false));
}

#[test]
fn test_parse_number() {
    assert_eq!(parse("42").unwrap(), JsonValue::Number(42.0));
    assert_eq!(parse("-3.14").unwrap(), JsonValue::Number(-3.14));
    assert_eq!(parse("1e10").unwrap(), JsonValue::Number(1e10));
    assert_eq!(parse("0").unwrap(), JsonValue::Number(0.0));
    assert_eq!(parse("-0").unwrap(), JsonValue::Number(0.0));
    assert_eq!(parse("1.5e-3").unwrap(), JsonValue::Number(0.0015));
}

#[test]
fn test_parse_string() {
    assert_eq!(parse(r#""hello""#).unwrap(), JsonValue::String("hello".into()));
    assert_eq!(parse(r#""say \"hi\"""#).unwrap(), JsonValue::String("say \"hi\"".into()));
    assert_eq!(parse(r#""line\nbreak""#).unwrap(), JsonValue::String("line\nbreak".into()));
    assert_eq!(parse(r#""tab\there""#).unwrap(), JsonValue::String("tab\there".into()));
}

#[test]
fn test_parse_unicode_escape() {
    // 基本 Unicode
    assert_eq!(parse(r#""\u0041""#).unwrap(), JsonValue::String("A".into()));
    // 中文
    assert_eq!(parse(r#""\u4f60\u597d""#).unwrap(), JsonValue::String("你好".into()));
    // Surrogate pair (😀 = U+1F600)
    assert_eq!(parse(r#""\uD83D\uDE00""#).unwrap(), JsonValue::String("😀".into()));
}

// ========== 复合类型 ==========

#[test]
fn test_parse_empty_array() {
    assert_eq!(parse("[]").unwrap(), JsonValue::Array(vec![]));
}

#[test]
fn test_parse_array() {
    let result = parse(r#"[1, "two", true, null]"#).unwrap();
    assert_eq!(result[0], JsonValue::Number(1.0));
    assert_eq!(result[1], JsonValue::String("two".into()));
    assert_eq!(result[2], JsonValue::Bool(true));
    assert_eq!(result[3], JsonValue::Null);
}

#[test]
fn test_parse_nested_array() {
    let result = parse("[[1, 2], [3, [4, 5]]]").unwrap();
    assert_eq!(result[1][1][0], JsonValue::Number(4.0));
}

#[test]
fn test_parse_empty_object() {
    let result = parse("{}").unwrap();
    assert!(result.is_object());
    assert_eq!(result.as_object().unwrap().len(), 0);
}

#[test]
fn test_parse_object() {
    let input = r#"{"name": "Rust", "year": 2015, "systems": true}"#;
    let result = parse(input).unwrap();
    assert_eq!(result["name"].as_str().unwrap(), "Rust");
    assert_eq!(result["year"].as_f64().unwrap(), 2015.0);
    assert_eq!(result["systems"].as_bool().unwrap(), true);
}

#[test]
fn test_parse_deeply_nested() {
    let input = r#"{"a": {"b": {"c": {"d": [1, 2, {"e": "deep"}]}}}}"#;
    let result = parse(input).unwrap();
    assert_eq!(result["a"]["b"]["c"]["d"][2]["e"].as_str().unwrap(), "deep");
}

// ========== 错误处理 ==========

#[test]
fn test_error_trailing_comma_array() {
    assert!(parse("[1, 2,]").is_err());
}

#[test]
fn test_error_trailing_comma_object() {
    assert!(parse(r#"{"a": 1,}"#).is_err());
}

#[test]
fn test_error_leading_zero() {
    assert!(parse("007").is_err());
}

#[test]
fn test_error_unterminated_string() {
    assert!(parse(r#""hello"#).is_err());
}

#[test]
fn test_error_invalid_token() {
    assert!(parse("undefined").is_err());
}

#[test]
fn test_error_extra_tokens() {
    assert!(parse("true false").is_err());
}

// ========== 序列化 ==========

#[test]
fn test_stringify_roundtrip() {
    let input = r#"{"array":[1,2,3],"bool":true,"null":null,"number":42,"string":"hello"}"#;
    let value = parse(input).unwrap();
    let output = stringify(&value);
    assert_eq!(input, output);
}

#[test]
fn test_stringify_pretty() {
    let value = parse(r#"{"a":1,"b":[2,3]}"#).unwrap();
    let pretty = stringify_pretty(&value, 2);
    assert!(pretty.contains("  \"a\": 1"));
    assert!(pretty.contains("  \"b\": [\n"));
}

#[test]
fn test_stringify_escaped_string() {
    let value = JsonValue::String("line1\nline2\ttab\"quote".into());
    let output = stringify(&value);
    assert_eq!(output, r#""line1\nline2\ttab\"quote""#);
}

// ========== json!() 宏 ==========

#[test]
fn test_json_macro_basic() {
    let v = json!(null);
    assert_eq!(v, JsonValue::Null);

    let v = json!(true);
    assert_eq!(v, JsonValue::Bool(true));

    let v = json!(42);
    assert_eq!(v.as_f64().unwrap(), 42.0);

    let v = json!("hello");
    assert_eq!(v.as_str().unwrap(), "hello");
}

#[test]
fn test_json_macro_complex() {
    let data = json!({
        "name": "test",
        "items": [1, 2, 3],
        "nested": {
            "active": true,
            "value": null
        }
    });

    assert_eq!(data["name"].as_str().unwrap(), "test");
    assert_eq!(data["items"][1].as_f64().unwrap(), 2.0);
    assert_eq!(data["nested"]["active"].as_bool().unwrap(), true);
    assert!(data["nested"]["value"].is_null());
}
```

---

## 第九章 进阶方向

当你完成了以上所有章节，JSON 库的核心功能已经齐备。以下是进一步提升的方向：

### 9.1 性能优化

| 方向 | 说明 |
|------|------|
| 零拷贝解析 | Lexer 返回 `&str` 切片而非 `String`，减少内存分配 |
| SIMD 加速空白跳过 | 用 SIMD 指令一次检查 16 字节是否为空白 |
| 流式解析 | 不构建完整 Token 列表，Lexer 按需产出 Token |
| `SmallVec` | 短数组用栈上分配替代堆分配 |

### 9.2 对接 serde 框架

实现 `serde::Serializer` 和 `serde::Deserializer` trait，让你的库可以被 `#[derive(Serialize, Deserialize)]` 使用：

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Config {
    name: String,
    version: f64,
}

// 你的库提供：
let config: Config = json_lib::from_str(r#"{"name":"app","version":1.0}"#)?;
let json_str = json_lib::to_string(&config)?;
```

这需要实现 serde 的 `Deserializer` trait（约 20 个方法），是一个很好的深度练习。

### 9.3 数字精度改进

用内部枚举区分整数和浮点数，避免大整数精度丢失：

```rust
enum JsonNumber {
    Integer(i64),
    Float(f64),
}
```

`i64` 可以精确表示到 \(2^{63}-1\)，而 `f64` 从 \(2^{53}\) 开始就会丢失精度。

### 9.4 符合 JSONTestSuite

[JSONTestSuite](https://github.com/nst/JSONTestSuite) 是一个全面的 JSON 解析器测试集，包含 300+ 测试用例，覆盖各种边界条件和规范合规性测试。通过这个测试集可以验证你的解析器的正确性。

### 9.5 实现 JSON Pointer（RFC 6901）

支持用路径字符串访问嵌套值：

```rust
let value = parse(r#"{"a": {"b": [1, 2, 3]}}"#)?;
let result = value.pointer("/a/b/1"); // => Some(Number(2.0))
```

---

## 附录 A：完整项目结构速查

```
json_lib/
├── Cargo.toml
├── src/
│   ├── lib.rs           # 公开 API + 声明宏
│   ├── value.rs         # JsonValue + Index/From/Display
│   ├── error.rs         # JsonError + JsonResult
│   ├── lexer.rs         # Token + Lexer (词法分析)
│   ├── parser.rs        # Parser (递归下降语法分析)
│   └── serializer.rs    # stringify / stringify_pretty
├── json_macro/          # [可选] 过程宏 crate
│   ├── Cargo.toml
│   └── src/lib.rs
└── tests/
    └── integration.rs   # 集成测试
```

## 附录 B：实现顺序建议

| 阶段 | 目标 | 预计时间 |
|------|------|----------|
| 1 | `error.rs` + `value.rs`（定义类型） | 0.5h |
| 2 | `lexer.rs`（不含 Unicode 转义） | 1-2h |
| 3 | `parser.rs`（递归下降） | 1-2h |
| 4 | 跑通第一个端到端测试 | 0.5h |
| 5 | `serializer.rs`（紧凑 + 美化） | 1h |
| 6 | Lexer 补充 Unicode 转义 + surrogate pairs | 1h |
| 7 | `json!()` 声明宏 | 0.5h |
| 8 | `json!()` 过程宏 | 1-2h |
| 9 | 完善测试 + 边界条件 | 1h |
| **合计** | | **7-10h** |

## 附录 C：参考资料

| 资源 | 链接 |
|------|------|
| RFC 8259 (JSON 规范) | https://tools.ietf.org/html/rfc8259 |
| JSON 可视化语法图 | https://www.json.org/json-en.html |
| serde_json 源码 | https://github.com/serde-rs/json |
| JSONTestSuite | https://github.com/nst/JSONTestSuite |
| The Rust Reference - Procedural Macros | https://doc.rust-lang.org/reference/procedural-macros.html |
| syn crate 文档 | https://docs.rs/syn |
| quote crate 文档 | https://docs.rs/quote |
