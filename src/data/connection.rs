use std::sync::{Arc, Mutex};

use async_std::{channel::Sender, net::TcpStream, stream::StreamExt};
use nu_protocol::{LabeledError, Record, ShellError, Span, Spanned, Value};
use tiberius::{AuthMethod, Client};

use super::{parse_value, ConnectionArgs};

#[derive(Debug, Clone)]
pub struct Connection {
    pub(crate) connection: Arc<Mutex<Client<TcpStream>>>,
    pub(crate) reference_count: usize,
}

impl Connection {
    pub fn new(connection: Arc<Mutex<Client<TcpStream>>>) -> Self {
        Self {
            connection,
            reference_count: 0,
        }
    }

    pub async fn close(&self) {
        let client_arc = self.connection.clone();
        let client_mutex = Arc::try_unwrap(client_arc).ok();
        if let Some(client) = client_mutex {
            let client = client.into_inner().unwrap();
            match client.close().await {
                Ok(_) => {
                    eprintln!("Connection: Closed connection");
                }
                Err(e) => {
                    eprintln!("Failed to close connection: {e}");
                }
            }
        }
    }

    pub async fn run_query<'a>(&self, query: Spanned<String>, sender: Sender<Value>) {
        let client_arc = self.connection.clone();
        let client_mutex = Arc::try_unwrap(client_arc).ok();
        if let Some(client) = client_mutex {
            let mut client = client.into_inner().unwrap();

            let stream = match client.simple_query(query.item).await {
                Ok(stream) => stream,
                Err(e) => {
                    panic!("Error: {}", e);
                }
            };

            let mut row_stream = stream.into_row_stream();
            while let Some(row) = row_stream.next().await {
                match row {
                    Ok(row) => {
                        let mut record = Record::new();

                        for (col, cell) in row.cells() {
                            match parse_value(cell) {
                                Ok(value) => {
                                    record.insert(col.name(), value);
                                }
                                Err(e) => {
                                    panic!("Error: {:?}", e);
                                }
                            }
                        }

                        if let Err(e) = sender.send(Value::record(record, Span::unknown())).await {
                            if sender.is_closed() {
                                return;
                            }
                            panic!("Error: {:?}", e);
                        }
                    }
                    Err(e) => {
                        panic!("Error: {}", e);
                    }
                }
            }
        } else {
            eprintln!("Failed to get connection reference");
        }
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    UserWithoutPassword(Span),
    LoginFailed(AuthMethod),
    SetupError(tiberius::error::Error),
    ConnectionError(tiberius::error::Error),
}

impl ConnectionError {
    pub fn to_shell_error(&self, args: &ConnectionArgs) -> ShellError {
        let error = match self {
            ConnectionError::LoginFailed(auth_method) => match auth_method {
                #[cfg(target_os = "windows")]
                tiberius::AuthMethod::Integrated => {
                    LabeledError::new("Login failed for integrated auth")
                }
                #[cfg(target_os = "windows")]
                tiberius::AuthMethod::Windows(_) => {
                    LabeledError::new("Windows auth not supported yet")
                }
                tiberius::AuthMethod::None => LabeledError::new("Login failed for none auth"),
                tiberius::AuthMethod::SqlServer(_) => LabeledError::new(format!(
                    "Login failed for user {:?}, password: <HIDDEN>",
                    args.user.as_ref().unwrap().as_str(),
                )),
                tiberius::AuthMethod::AADToken(_) => {
                    LabeledError::new("AADToken auth not supported")
                }
                #[cfg(target_os = "linux")]
                _ => todo!(),
            },
            ConnectionError::UserWithoutPassword(span) => LabeledError::new("Invalid credentials")
                .with_label("User specified without password", *span),
            ConnectionError::SetupError(error) => LabeledError::new(format!(
                "Error while setting up connection: {}",
                error.to_string()
            )),
            ConnectionError::ConnectionError(error) => LabeledError::new(format!(
                "Error while connecting to database: {}",
                error.to_string()
            )),
        };

        ShellError::LabeledError(Box::new(error))
    }
}
