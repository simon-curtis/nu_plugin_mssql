use nu_protocol::{LabeledError, Span, Value};

pub const DEFAULT_BUFFER_SIZE: usize = 10;

pub struct ConnectionSettings {
    pub server: Option<Value>,
    pub instance: Option<Value>,
    pub database: Option<Value>,
    pub user: Option<Value>,
    pub password: Option<Value>,
    pub trust_cert: Option<Span>,
    pub buffer_size: usize,
}

impl ConnectionSettings {
    pub fn from_call(call: &nu_plugin::EvaluatedCall) -> Result<ConnectionSettings, LabeledError> {
        let mut args = ConnectionSettings {
            server: None,
            instance: None,
            database: None,
            user: None,
            password: None,
            buffer_size: DEFAULT_BUFFER_SIZE,
            trust_cert: call.get_flag_span("trust-cert"),
        };

        for (name, value) in call.named.iter() {
            match name.item.as_str() {
                "server" => args.server = value.clone(),
                "instance" => args.instance = value.clone(),
                "database" => args.database = value.clone(),
                "user" => args.user = value.clone(),
                "password" => args.password = value.clone(),
                "buffer_size" => {
                    args.buffer_size = match value {
                        Some(Value::Int { val, .. }) => val.clone() as usize,
                        Some(other) => {
                            return Err(LabeledError::new(format!(
                                "Invalid buffer size type {:?}",
                                other
                            )))
                        }
                        _ => 10,
                    }
                }
                _ => {}
            }
        }

        Ok(args)
    }
}
