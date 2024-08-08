
use std::hash::{Hash, Hasher};
use nu_protocol::{LabeledError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::DEFAULT_BUFFER_SIZE;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionArgs {
    pub server: Option<Value>,
    pub instance: Option<Value>,
    pub database: Option<Value>,
    pub user: Option<Value>,
    #[serde(skip)]
    pub password: Option<Value>,
    pub trust_cert: Option<Span>,
    pub buffer_size: usize,
    pub reference_count: usize,
}

impl PartialEq for ConnectionArgs {
    fn eq(&self, other: &Self) -> bool {
        self.server == other.server
            && self.instance == other.instance
            && self.database == other.database
            && self.user == other.user
            && self.password == other.password
            && self.trust_cert == other.trust_cert
            && self.buffer_size == other.buffer_size
    }
}

impl Eq for ConnectionArgs {}

impl Hash for ConnectionArgs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(server) = &self.server {
            server.to_debug_string().hash(state);
        }

        if let Some(instance) = &self.instance {
            instance.to_debug_string().hash(state);
        }

        if let Some(database) = &self.database {
            database.to_debug_string().hash(state); 
        }

        if let Some(user) = &self.user {
            user.to_debug_string().hash(state);
        }

        if let Some(password) = &self.password {
            password.to_debug_string().hash(state); 
        }

        self.trust_cert.is_some().hash(state);
        self.buffer_size.hash(state);
    }
}


impl ConnectionArgs {
    pub fn from_call(call: &nu_plugin::EvaluatedCall) -> Result<ConnectionArgs, LabeledError> {
        let mut args = ConnectionArgs {
            server: None,
            instance: None,
            database: None,
            user: None,
            password: None,
            buffer_size: DEFAULT_BUFFER_SIZE,
            trust_cert: call.get_flag_span("trust-cert"),
            reference_count: 0
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
                    _ => {}
                },
                None => {}
            };
        }

        Ok(args)
    }
    
    pub(crate) fn as_ref(&self) -> &ConnectionArgs {
        self
    }
}