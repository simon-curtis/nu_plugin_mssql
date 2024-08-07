use crate::data::{
    connection::{create_client, ConnectionError},
    query_args::{QueryArgs, QuerySource},
};
use async_std::task;
use nu_plugin::PluginCommand;
use nu_protocol::{
    IntoInterruptiblePipelineData, IntoSpanned, LabeledError, PipelineData, ShellError, Signals,
    Signature, Span, Spanned, SyntaxShape, Value,
};

use crate::{
    data::db::{run_query, TableIterator},
    MssqlPlugin, DEFAULT_BUFFER_SIZE,
};

pub struct Query;

impl PluginCommand for Query {
    type Plugin = MssqlPlugin;

    fn name(&self) -> &str {
        "mssql query"
    }

    fn usage(&self) -> &str {
        "Run a query against a MSSQL database"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
            .named(
                "query",
                SyntaxShape::String,
                "The query to run against the database",
                Some('q'),
            )
            .named(
                "file",
                SyntaxShape::Filepath,
                "The path to a file containing the query",
                Some('f'),
            )
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
                "The user to connect as, default: sa",
                Some('u'),
            )
            .named(
                "password",
                SyntaxShape::String,
                "The password to connect with",
                Some('p'),
            )
            .named(
                "row-buffer",
                SyntaxShape::Int,
                format!(
                    "The max number of rows to buffer ahead of the pipeline, default: {}",
                    DEFAULT_BUFFER_SIZE
                ),
                Some('b'),
            )
            .switch("trust-cert", "Trust the server certificate", Some('t'))
            .category(nu_protocol::Category::Database)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let args = QueryArgs::from_call(call)?;
        let call_head = call.head;
        let query = get_query(&args)?;

        let (sender, receiver) = async_std::channel::bounded(args.buffer_size);
        task::spawn(async move {
            match create_client(&args).await {
                Ok(mut client) => run_query(query, &mut client, sender).await,
                Err(e) => handle_err(e, &args, call_head, sender).await,
            }
        });

        let iterator = TableIterator::new(receiver);
        Ok(iterator.into_pipeline_data(call.head, Signals::empty()))
    }
}

fn get_query(args: &QueryArgs) -> Result<Spanned<String>, LabeledError> {
    match args.source.as_ref() {
        Some(QuerySource::Query(query, span)) => Ok(query.clone().into_spanned(span.clone())),
        Some(QuerySource::File(file, span)) => match std::fs::read_to_string(file) {
            Ok(query) => Ok(query.into_spanned(span.clone())),
            Err(e) => Err(LabeledError::new(format!(
                "Error reading file {}: {}",
                file, e
            ))),
        },
        None => Err(LabeledError::new("No query specified")),
    }
}

async fn handle_err(
    error: ConnectionError,
    args: &QueryArgs,
    call_head: Span,
    sender: async_std::channel::Sender<Value>,
) {
    let value = match error {
        ConnectionError::LoginFailed(auth_method) => {
            let error = match auth_method {
                #[cfg(target_os = "windows")]
                tiberius::AuthMethod::Integrated => {
                    LabeledError::new("Login failed for integrated auth")
                }
                tiberius::AuthMethod::None => LabeledError::new("Login failed for none auth"),
                tiberius::AuthMethod::SqlServer(_) => LabeledError::new(format!(
                    "Login failed for user {:?}, password: <HIDDEN>",
                    args.user.as_ref().unwrap().as_str(),
                )),
                tiberius::AuthMethod::AADToken(_) => panic!("AADToken auth not supported"),
            };
            let error = ShellError::LabeledError(Box::new(error));
            Value::error(error, call_head)
        }
        ConnectionError::UserWithoutPassword(span) => {
            let error = LabeledError::new("Invalid credentials")
                .with_label("User specified without password", span);
            let error = ShellError::LabeledError(Box::new(error));
            Value::error(error, call_head)
        }
        ConnectionError::SetupError(error) => {
            let error = LabeledError::new(format!(
                "Error while setting up connection: {}",
                error.to_string()
            ));
            let error = ShellError::LabeledError(Box::new(error));
            Value::error(error, call_head)
        }
        ConnectionError::ConnectionError(error) => {
            let error = LabeledError::new(format!(
                "Error while connecting to database: {}",
                error.to_string()
            ));
            let error = ShellError::LabeledError(Box::new(error));
            Value::error(error, call_head)
        }
    };

    let _ = sender.send(value).await;
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
