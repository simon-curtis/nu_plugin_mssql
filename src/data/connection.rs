use async_std::net::TcpStream;
use nu_protocol::{LabeledError, Span, Value};
use serde::{Deserialize, Serialize};
use tiberius::{AuthMethod, Client, Config, SqlBrowser};

use crate::DEFAULT_BUFFER_SIZE;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn to_config(&self) -> anyhow::Result<Config, LabeledError> {
        let mut config = Config::new();

        if let Some(server) = &self.server {
            config.host(server.as_str().unwrap());
        } else {
            config.host("localhost");
        }

        if let Some(database) = &self.database {
            config.database(database.as_str().unwrap());
        } else {
            config.database("master");
        }

        if let Some(instance) = &self.instance {
            config.instance_name(instance.as_str().unwrap());
        } else {
            config.port(1433)
        }

        match self.get_auth_method() {
            Ok(auth_method) => config.authentication(auth_method),
            Err(e) => return Err(e),
        }

        if self.trust_cert.is_some() {
            config.trust_cert();
        }

        Ok(config)
    }

    pub async fn create_client(
        &self,
        args: &ConnectionSettings,
    ) -> anyhow::Result<Client<TcpStream>, LabeledError> {
        let config = args.to_config()?;

        let tcp = match self.create_stream(&config).await {
            Ok(tcp) => tcp,
            Err(e) => return Err(e),
        };

        match Client::connect(config, tcp).await {
            Ok(client) => Ok(client),
            Err(e) => match &args.server {
                Some(server) => Err(LabeledError::new("Failed to connect to server")
                    .with_label("here", server.span())
                    .with_label(e.to_string(), Span::unknown())),
                None => Err(LabeledError::new("Failed to connect to instance")
                    .with_label(e.to_string(), Span::unknown())),
            },
        }
    }

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

    pub async fn create_stream(&self, config: &Config) -> anyhow::Result<TcpStream, LabeledError> {
        let tcp = match &self.instance {
            Some(instance) => match TcpStream::connect_named(config).await {
                Ok(tcp) => tcp,
                Err(e) => {
                    return Err(LabeledError::new("Failed to connect to instance")
                        .with_label("here", instance.span())
                        .with_label(e.to_string(), Span::unknown()))
                }
            },
            None => match TcpStream::connect(config.get_addr()).await {
                Ok(tcp) => tcp,
                Err(e) => {
                    return match &self.server {
                        Some(server) => Err(LabeledError::new("Failed to connect to server")
                            .with_label("here", server.span())
                            .with_label(e.to_string(), Span::unknown())),
                        None => Err(LabeledError::new("Failed to connect to server")
                            .with_label(e.to_string(), Span::unknown())),
                    }
                }
            },
        };

        tcp.set_nodelay(true).unwrap();
        Ok(tcp)
    }

    fn get_auth_method(&self) -> Result<AuthMethod, LabeledError> {
        match (&self.user, &self.password) {
            (Some(Value::String { val: user, .. }), Some(Value::String { val: password, .. })) => {
                Ok(AuthMethod::sql_server(user, password))
            }
            (Some(username), None) => Err(LabeledError::new("Invalid credentials")
                .with_label("Username specified without password", username.span())),
            (None, Some(password)) => Err(LabeledError::new("Invalid credentials")
                .with_label("Password specified without username", password.span())),
            #[cfg(target_os = "windows")]
            _ => Ok(AuthMethod::Integrated),
            #[cfg(target_os = "linux")]
            _ => Ok(AuthMethod::None),
        }
    }
}
