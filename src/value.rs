use std::collections::BTreeMap;
use std::fmt;
use std::ops::Index;

// Json值的类型
#[derive(Debug, PartialEq, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, Value>),
}

pub struct JsonParser {
    input: Vec<char>,
    index: usize,
}

impl JsonValue {
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }
    
    pub fn is_bool(&self) -> bool {
        matches!(self, JsonValue::bool(_))
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
            JsonValue::Number(n) => Some(*b),
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

// value["Key"]访问Object的字段
impl Index<&str> for JsonValue {
    type output = JsonValue;

    fn index(&self, key: &str) -> &JsonValue {
        match self {
            JsonValue::Object(map) => map.get(key).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}

// value[0]访问Array的元素
impl Index<usize> for JsonValue {
    type output = JsonValue;
    
    fn index(&self, index: usize) -> &JsonValue {
        match self {
            JsonValue::Array(arr) => arr.get(index).unwrap_or(&JsonValue::Null),
            _ => &JsonValue::Null,
        }
    }
}

impl From<bool> for JsonValue {
    fn from(b: bool) ->Self { JsonValue::Bool(b)}
}

impl From<f64> for JsonValue {
    fn from(n: f64) ->Self {JsonValue::Number(n)}
}

impl From<i64> for JsonValue {
    fn from(n: i64) ->Self {JsonValue::Number(n as f64)}
}

impl From<&str> for JsonValue {
    fn from(s: &str) ->Self {JsonValue::String(s.to_string())}
}

impl From<String> for JsonValue {
    fn from(s: String) -> Self {JsonValue::String(s)}
}

impl <T: Into<JsonValue>> From<Vec<T>> for JsonValue {
    fn from (v: Vec<T>) -> Self {
        JsonValue::Array(v.into_ite().map(|x| x.into()).collect())
    }
}

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
                    if i > 0 {write!(f, ",")?;}
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            JsonValue::Object(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "\"{}\":{}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}
