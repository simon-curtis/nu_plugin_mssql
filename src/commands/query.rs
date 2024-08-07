use crate::data::{create_client, ConnectionArgs, MssqlClient, QuerySource};
use async_std::task;
use nu_plugin::PluginCommand;
use nu_protocol::{
    IntoInterruptiblePipelineData, IntoSpanned, LabeledError, PipelineData, ShellError, Signals, Signature, Spanned, SyntaxShape, Type, Value
};

use crate::{
    data::TableIterator,
    MssqlPlugin, DEFAULT_BUFFER_SIZE,
};

use super::handle_err;

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
            .input_output_type(
                Type::Custom("MssqlClient".into()),
                Type::table(),
            )
            .switch("trust-cert", "Trust the server certificate", Some('t'))
            .category(nu_protocol::Category::Database)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let args = ConnectionArgs::from_call(call)?;
        let query = QuerySource::from_call(call)?;
        let call_head = call.head;
        let query = get_query(&query)?;

        let (sender, receiver) = async_std::channel::bounded(args.buffer_size);
        task::spawn(async move {
            let client = match try_get_client(input) {
                Some(client) => client,
                _ => match create_client(&args).await {
                    Ok(client) => {
                        MssqlClient::new(args.clone(), client)
                    },
                    Err(e) => {
                        let err=  handle_err(e, &args);
                        let err = ShellError::LabeledError(Box::new(err));
                        let value = Value::error(err, call_head);
                        _ = sender.send(value).await;
                        return;
                    }
                }
            };

            client.run_query(query, sender).await;
        });

        let iterator = TableIterator::new(receiver);
        Ok(iterator.into_pipeline_data(call.head, Signals::empty()))
    }
}

fn try_get_client(input: PipelineData) -> Option<MssqlClient> {
    match input {
        PipelineData::Value(value, _) => match value {
            Value::Custom { val , ..} => match val.as_any().downcast_ref::<MssqlClient>() {
                Some(client) => Some(client.clone()),
                None => None,
            },
            _ => None,
        },
        _ => None,
    } 
}

fn get_query(query: &QuerySource) -> Result<Spanned<String>, LabeledError> {
    match query {
        QuerySource::Query(query, span) => Ok(query.clone().into_spanned(span.clone())),
        QuerySource::File(file, span) => match std::fs::read_to_string(file) {
            Ok(query) => Ok(query.into_spanned(span.clone())),
            Err(e) => Err(LabeledError::new(format!(
                "Error reading file {}: {}",
                file, e
            ))),
        }
    }
}

#[test]
fn test_basic_connection() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::Span;
    let mut plugin_test = PluginTest::new("mssql", MssqlPlugin.into())?;

    // Now lets add a positional argument
    let output = plugin_test
        .eval("mssql -i SQL2022 -d master -t \"SELECT 1 AS [Count] UNION SELECT 2 AS [Count]\"")?
        .into_value(Span::unknown())?;

    println!("Result: {:?}", output);
    Ok(())
}
