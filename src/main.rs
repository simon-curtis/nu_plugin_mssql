use std::thread;

use async_std::channel::{Receiver, Sender, TrySendError};
use async_std::net::TcpStream;
use async_std::stream::StreamExt;
use async_std::task;
use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin::{Plugin, PluginCommand};
use nu_protocol::{
    IntoInterruptiblePipelineData, LabeledError, PipelineData, Record, Signals, Signature, Span,
    Spanned, SyntaxShape, Type, Value,
};
use tiberius::time::chrono::{DateTime, FixedOffset, NaiveDateTime};
use tiberius::{AuthMethod, Client, ColumnData, Config, FromSql, Query, SqlBrowser};

struct MssqlPlugin;
struct MssqlPluginQuery;

async fn create_client(args: &Args) -> anyhow::Result<Client<TcpStream>, LabeledError> {
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

    match (&args.user, &args.password) {
        (Some(Value::String { val: user, .. }), Some(Value::String { val: password, .. })) => {
            config.authentication(AuthMethod::sql_server(user, password));
        }
        (Some(username), None) => {
            return Err(LabeledError::new("Invalid credentials")
                .with_label("Username specified without password", username.span()));
        }
        (None, Some(password)) => {
            return Err(LabeledError::new("Invalid credentials")
                .with_label("Password specified without username", password.span()));
        }
        _ => config.authentication(AuthMethod::Integrated),
    };

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
            Err(e) => {
                return Err(LabeledError::new("Failed to connect to server")
                    .with_label("here", args.server.as_ref().unwrap().span())
                    .with_label(e.to_string(), Span::unknown()))
            }
        },
    };

    tcp.set_nodelay(true).unwrap();
    match Client::connect(config, tcp).await {
        Ok(client) => Ok(client),
        Err(e) => Err(LabeledError::new("Failed to connect to server")
            .with_label("here", args.server.as_ref().unwrap().span())
            .with_label(e.to_string(), Span::unknown())),
    }
}

impl Plugin for MssqlPlugin {
    fn version(&self) -> String {
        "0.0.1".to_string()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(MssqlPluginQuery)]
    }
}

fn parse_value(data: &ColumnData<'static>) -> anyhow::Result<Value> {
    match data {
        ColumnData::Binary(Some(val)) => Ok(Value::binary(val.as_ref(), Span::unknown())),
        ColumnData::String(Some(val)) => Ok(Value::string(val.as_ref(), Span::unknown())),
        ColumnData::I32(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::F32(Some(val)) => Ok(Value::float(*val as f64, Span::unknown())),
        ColumnData::DateTime2(Some(_)) => {
            let naive = NaiveDateTime::from_sql(data)?.expect("failed to parse datetime");
            let date_time = DateTime::<FixedOffset>::from_utc(naive, FixedOffset::east(0));
            Ok(Value::date(date_time, Span::unknown()))
        }
        _ => Ok(Value::nothing(Span::unknown())),
    }
}

struct Args {
    server: Option<Value>,
    instance: Option<Value>,
    database: Option<Value>,
    user: Option<Value>,
    password: Option<Value>,
    trust_cert: Option<Span>,
}

impl Args {
    fn from_call(call: &nu_plugin::EvaluatedCall) -> Args {
        let mut args = Args {
            server: None,
            instance: None,
            database: None,
            user: None,
            password: None,
            trust_cert: call.get_flag_span("trust_cert"),
        };

        for (name, value) in call.named.iter() {
            match name.item.as_str() {
                "server" => args.server = value.clone(),
                "instance" => args.instance = value.clone(),
                "database" => args.database = value.clone(),
                "user" => args.user = value.clone(),
                "password" => args.password = value.clone(),
                _ => {}
            }
        }

        args
    }
}

struct TableIterator {
    receiver: Receiver<Record>,
}

impl TableIterator {
    fn new(receiver: Receiver<Record>) -> TableIterator {
        TableIterator { receiver }
    }
}

impl<'a> Iterator for TableIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv_blocking() {
            Ok(record) => Some(Value::record(record, Span::unknown())),
            Err(_) => None,
        }
    }
}

async fn run_query(query: String, args: Args, sender: Sender<Record>) {
    let mut client = match create_client(&args).await {
        Ok(client) => client,
        Err(e) => {
            panic!("Error: {}", e);
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

impl PluginCommand for MssqlPluginQuery {
    type Plugin = MssqlPlugin;

    fn name(&self) -> &str {
        "mssql"
    }

    fn usage(&self) -> &str {
        "A plugin for connecting to a MSSQL database"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .required("query", SyntaxShape::String, "The query to run")
            .named(
                "server",
                SyntaxShape::String,
                "The server to connect to, default: localhost",
                Some('s'),
            )
            .named(
                "instance",
                SyntaxShape::String,
                "The server instance to connect to",
                Some('i'),
            )
            .named(
                "database",
                SyntaxShape::String,
                "The database to connect to, default: master",
                Some('d'),
            )
            .named(
                "user",
                SyntaxShape::String,
                "The user to connect as",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::String,
                "The password to connect with",
                Some('p'),
            )
            .switch("trust_cert", "Trust the server certificate", Some('t'))
            .input_output_type(Type::Nothing, Type::ListStream)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let query: Spanned<String> = call.req(0)?;
        let args = Args::from_call(call);

        let (sender, receiver) = async_std::channel::bounded(1);

        task::spawn(run_query(query.item.clone(), args, sender));

        let iterator: TableIterator = TableIterator::new(receiver);
        Ok(iterator.into_pipeline_data(Span::unknown(), Signals::empty()))
    }
}

fn main() {
    serve_plugin(&MssqlPlugin, JsonSerializer)
}

#[test]
fn test_basic_connection() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    let mut plugin_test = PluginTest::new("mssql", MssqlPlugin.into())?;

    // Now lets add a positional argument
    let output = plugin_test
        .eval("mssql -i SQL2022 -d master -t \"SELECT 1 AS [Count] UNION SELECT 2 AS [Count]\"")?
        .into_value(Span::unknown())?;

    println!("Result: {:?}", output);
    Ok(())
}
