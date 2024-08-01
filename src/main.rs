use async_std::task;
use db::{run_query, TableIterator};
use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin::{Plugin, PluginCommand};
use nu_protocol::{
    IntoInterruptiblePipelineData, LabeledError, PipelineData, Signals, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};

mod db;

struct MssqlPlugin;
struct MssqlPluginQuery;

impl Plugin for MssqlPlugin {
    fn version(&self) -> String {
        "0.0.1".to_string()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(MssqlPluginQuery)]
    }
}

const DEFAULT_BUFFER_SIZE: usize = 10;

struct Args {
    server: Option<Value>,
    instance: Option<Value>,
    database: Option<Value>,
    user: Option<Value>,
    password: Option<Value>,
    trust_cert: Option<Span>,
    buffer_size: usize,
}

impl Args {
    fn from_call(call: &nu_plugin::EvaluatedCall) -> Result<Args, LabeledError> {
        let mut args = Args {
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
        let args = Args::from_call(call)?;
        let (sender, receiver) = async_std::channel::bounded(args.buffer_size);
        task::spawn(run_query(query.item.clone(), args, sender));

        let iterator = TableIterator::new(receiver);
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
