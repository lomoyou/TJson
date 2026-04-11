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