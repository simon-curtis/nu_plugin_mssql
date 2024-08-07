use nu_protocol::{LabeledError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::DEFAULT_BUFFER_SIZE;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryArgs {
    pub server: Option<Value>,
    pub instance: Option<Value>,
    pub database: Option<Value>,
    pub user: Option<Value>,
    pub password: Option<Value>,
    pub trust_cert: Option<Span>,
    pub buffer_size: usize,
    pub source: Option<QuerySource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuerySource {
    Query(String, Span),
    File(String, Span),
}

impl QueryArgs {
    pub fn from_call(call: &nu_plugin::EvaluatedCall) -> Result<QueryArgs, LabeledError> {
        let mut args = QueryArgs {
            server: None,
            instance: None,
            database: None,
            user: None,
            password: None,
            buffer_size: DEFAULT_BUFFER_SIZE,
            trust_cert: call.get_flag_span("trust-cert"),
            source: None,
        };

        for (name, value) in call.named.iter() {
            match value {
                Some(value) => match name.item.as_str() {
                    "server" => args.server = Some(value.clone()),
                    "instance" => args.instance = Some(value.clone()),
                    "database" => args.database = Some(value.clone()),
                    "user" => args.user = Some(value.clone()),
                    "password" => args.password = Some(value.clone()),
                    "buffer_size" => {
                        args.buffer_size = match value {
                            Value::Int { val, .. } => val.clone() as usize,
                            other => panic!("Invalid buffer size type {:?}", other),
                        }
                    }
                    "query" if args.source.is_none() => {
                        args.source = Some(QuerySource::Query(
                            value.as_str().unwrap().to_string(),
                            value.span(),
                        ))
                    }
                    "file" if args.source.is_none() => {
                        args.source = Some(QuerySource::File(
                            value.as_str().unwrap().to_string(),
                            value.span(),
                        ))
                    }
                    _ => {}
                },
                None => {}
            };
        }

        Ok(args)
    }
}
