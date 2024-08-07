use nu_protocol::{LabeledError, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuerySource {
    Query(String, Span),
    File(String, Span),
}

impl QuerySource {
    pub fn from_call(call: &nu_plugin::EvaluatedCall) -> Result<QuerySource, LabeledError> {
        for (name, value) in call.named.iter() {
            match value {
                Some(value) => match name.item.as_str() {
                    "query" => {
                        return Ok(QuerySource::Query(
                            value.as_str().unwrap().to_string(),
                            value.span(),
                        ))
                    }
                    "file" => {
                        return Ok(QuerySource::File(
                            value.as_str().unwrap().to_string(),
                            value.span(),
                        ))
                    }
                    _ => {}
                },
                None => {}
            };
        }

        Err(LabeledError::new("No query specified"))
    }
}