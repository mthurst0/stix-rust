use super::errors::MyError;

pub struct ContentBinding {
    pub binding_id: String,
    pub subtype_id: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ResponseType {
    Full,
    CountOnly,
}

impl ResponseType {
    pub fn parse(v: &str) -> Result<ResponseType, MyError> {
        match v {
            "FULL" => Ok(ResponseType::Full),
            "COUNT_ONLY" => Ok(ResponseType::CountOnly),
            _ => Err(MyError(format!("could not parse response type: {}", v))),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            ResponseType::Full => "FULL",
            ResponseType::CountOnly => "COUNT_ONLY",
        }
    }
}
