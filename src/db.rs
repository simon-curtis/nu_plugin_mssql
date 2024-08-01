use std::any::Any;

use async_std::{
    channel::{Receiver, Sender},
    net::TcpStream,
    stream::StreamExt,
};
use nu_protocol::{LabeledError, Record, Span, Value};
use tiberius::{
    time::chrono::{DateTime, FixedOffset, NaiveDateTime},
    AuthMethod, Client, ColumnData, Config, FromSql, Query, SqlBrowser,
};

use crate::query::ConnectionSettings;

pub fn get_auth_method(args: &ConnectionSettings) -> Result<AuthMethod, LabeledError> {
    match (&args.user, &args.password) {
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

pub async fn create_client(
    args: &ConnectionSettings,
) -> anyhow::Result<Client<TcpStream>, LabeledError> {
    let mut config = Config::new();

    if let Some(server) = &args.server {
        config.host(server.as_str().unwrap());
    } else {
        config.host("localhost");
    }

    if let Some(database) = &args.database {
        config.database(database.as_str().unwrap());
    } else {
        config.database("master");
    }

    if let Some(instance) = &args.instance {
        config.instance_name(instance.as_str().unwrap());
    } else {
        config.port(1433)
    }

    match get_auth_method(&args) {
        Ok(auth_method) => config.authentication(auth_method),
        Err(e) => return Err(e),
    }

    if args.trust_cert.is_some() {
        config.trust_cert();
    }

    let tcp = match args.instance {
        Some(_) => match TcpStream::connect_named(&config).await {
            Ok(tcp) => tcp,
            Err(e) => {
                return Err(LabeledError::new("Failed to connect to instance")
                    .with_label("here", args.instance.as_ref().unwrap().span())
                    .with_label(e.to_string(), Span::unknown()))
            }
        },
        None => match TcpStream::connect(config.get_addr()).await {
            Ok(tcp) => tcp,
            Err(e) if args.server.is_some() => {
                return Err(LabeledError::new("Failed to connect to server")
                    .with_label("here", args.server.as_ref().unwrap().span())
                    .with_label(e.to_string(), Span::unknown()))
            }
            Err(e) => {
                return Err(LabeledError::new("Failed to connect to server")
                    .with_label(e.to_string(), Span::unknown()))
            }
        },
    };

    tcp.set_nodelay(true).unwrap();
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

pub fn parse_value(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
    match data {
        ColumnData::Binary(Some(val)) => Ok(Value::binary(val.as_ref(), Span::unknown())),
        ColumnData::String(Some(val)) => Ok(Value::string(val.as_ref(), Span::unknown())),
        ColumnData::I32(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::F32(Some(val)) => Ok(Value::float(*val as f64, Span::unknown())),
        ColumnData::DateTime2(Some(_)) => parse_date(data),
        other => Err(LabeledError::new(format!(
            "Failed to parse value: {:?}",
            other
        ))),
    }
}

fn parse_date(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
    match NaiveDateTime::from_sql(data) {
        Ok(naive) => match naive {
            Some(naive) => match FixedOffset::east_opt(0) {
                Some(offset) => {
                    let date_time =
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(naive, offset);
                    Ok(Value::date(date_time, Span::unknown()))
                }
                None => Err(LabeledError::new("Failed to parse datetime")
                    .with_label("Invalid datetime", Span::unknown())),
            },
            None => Err(LabeledError::new("Failed to parse datetime")
                .with_label("Invalid datetime", Span::unknown())),
        },
        Err(e) => Err(LabeledError::new("Failed to parse datetime")
            .with_label(e.to_string(), Span::unknown())),
    }
}

pub struct TableIterator {
    receiver: Receiver<Record>,
}

impl TableIterator {
    pub fn new(receiver: Receiver<Record>) -> Self {
        Self { receiver }
    }
}

impl Iterator for TableIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv_blocking() {
            Ok(record) => Some(Value::record(record, Span::unknown())),
            Err(_) => None,
        }
    }
}

pub async fn run_query(query: String, args: ConnectionSettings, sender: Sender<Record>) {
    let mut client = match create_client(&args).await {
        Ok(client) => client,
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    };

    let select = Query::new(query);
    let stream = match select.query(&mut client).await {
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

                if let Err(e) = sender.send(record).await {
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
