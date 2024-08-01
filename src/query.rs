use async_std::task;
use nu_plugin::PluginCommand;
use nu_protocol::{
    IntoInterruptiblePipelineData, LabeledError, PipelineData, Signals, Signature, Span, Spanned,
    SyntaxShape,
};

use crate::{
    data::connection::{ConnectionSettings, DEFAULT_BUFFER_SIZE},
    data::db::{run_query, TableIterator},
    MssqlPlugin,
};

pub struct MssqlPluginQuery;

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
            .named(
                "query",
                SyntaxShape::String,
                "The query to run against the database",
                Some('q'),
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
                "The user to connect as",
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
        let query: Spanned<String> = call.req(0)?;
        let args = ConnectionSettings::from_call(call)?;

        for (name, _) in call.named.iter() {
            match name.item.as_str() {
                "query" => {
                    let (sender, receiver) = async_std::channel::bounded(args.buffer_size);
                    task::spawn(run_query(query.item.clone(), args, sender));

                    let iterator = TableIterator::new(receiver);
                    return Ok(iterator.into_pipeline_data(Span::unknown(), Signals::empty()));
                }
                _ => {}
            }
        }

        Err(LabeledError::new("No command specified"))
    }
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
