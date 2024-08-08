use crate::data::{ConnectionArgs, QuerySource};
use async_std::task;
use nu_plugin::PluginCommand;
use nu_protocol::{
    IntoInterruptiblePipelineData, IntoPipelineData, IntoSpanned, LabeledError, PipelineData,
    Signals, Signature, Spanned, SyntaxShape, Type, Value,
};

use crate::{data::TableIterator, MssqlPlugin, DEFAULT_BUFFER_SIZE};

pub struct Query;

impl<'a> PluginCommand for Query {
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
            .input_output_type(Type::Custom("MssqlClient".into()), Type::table())
            .switch("trust-cert", "Trust the server certificate", Some('t'))
            .category(nu_protocol::Category::Database)
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, nu_protocol::LabeledError> {
        let args = ConnectionArgs::from_call(call)?;
        let query = QuerySource::from_call(call)?;
        let query = get_query(&query)?;
        let (sender, receiver) = async_std::channel::bounded(args.as_ref().buffer_size);

        let connection = task::block_on(async {
            match plugin.connection_pool.get(&args, true) {
                Ok(Some(connection)) => {
                    eprintln!("Connection pool returned existing connection");
                    Ok(connection)
                }
                Ok(None) => match plugin.connection_pool.create_connection(engine, args).await {
                    Ok(connection) => {
                        eprintln!("Connection pool created new connection");
                        Ok(connection)
                    }
                    Err(e) => Err(e),
                },
                Err(e) => Err(e),
            }
        });

        match connection {
            Ok(connection) => {
                task::spawn(async move {
                    _ = &connection.run_query(query, sender).await;
                });
            }
            Err(e) => {
                let value = Value::error(e, call.head);
                return Ok(value.into_pipeline_data_with_metadata(input.metadata()));
            }
        }

        let iterator = TableIterator::new(receiver);
        Ok(iterator.into_pipeline_data(call.head, Signals::empty()))
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
        },
    }
}

#[test]
fn test_basic_connection() -> Result<(), nu_protocol::ShellError> {
    use nu_plugin_test_support::PluginTest;
    use nu_protocol::Span;
    let mut plugin_test = PluginTest::new("mssql", MssqlPlugin::new().into())?;

    // Now lets add a positional argument
    let output = plugin_test
        .eval("mssql -i SQL2022 -d master -t \"SELECT 1 AS [Count] UNION SELECT 2 AS [Count]\"")?
        .into_value(Span::unknown())?;

    println!("Result: {:?}", output);
    Ok(())
}
