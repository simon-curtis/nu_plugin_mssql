use std::sync::Arc;
use async_std::{channel::Sender, net::TcpStream, stream::StreamExt};
use nu_protocol::{CustomValue, Record, ShellError, Span, Spanned, Value};
use serde::{Deserialize, Serialize};
use tiberius::Client;

use super::{parse_value, ConnectionArgs};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MssqlClient {
    args: ConnectionArgs,
    #[serde(skip)]
    pub client: Option<Arc<Client<TcpStream>>>,
}

impl MssqlClient {
    pub fn new(args: ConnectionArgs, client: Client<TcpStream>) -> Self {
        Self {
            args,
            client: Some(Arc::new(client)),
        }
    }

    pub fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }

    pub fn notify(&self) {
        eprintln!("MssqlClient was dropped: {:?}", self.client);
    }

    pub async fn run_query<'a>(
        &self,
        query: Spanned<String>,
        sender: Sender<Value>,
    ) {
        let client = match &self.client {
            Some(client) => Arc::clone(client),
            None => {
                eprintln!("Client appears to have been dropped");
                return
            },
        };

        let mut client_ref = match Arc::try_unwrap(client) {
            Ok(client) => client,
            Err(_) => {
                eprintln!("Failed to unwrap Rc");
                return;
            }
        };

        let stream = match client_ref.simple_query(query.item).await {
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
    }
}

#[typetag::serde]
impl CustomValue for MssqlClient {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        "MssqlClient".into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::string(
            format!("{:?}", self.args),
            span,
        ))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn notify_plugin_on_drop(&self) -> bool {
        // This is what causes Nushell to let us know when the value is dropped
        true
    }
}
