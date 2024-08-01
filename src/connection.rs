use async_std::task;
use nu_plugin::SimplePluginCommand;
use nu_protocol::{Signature, Span, SyntaxShape, Value};

use crate::{
    data::connection::{ConnectionSettings, DEFAULT_BUFFER_SIZE},
    models::StreamWrapper,
    MssqlPlugin,
};

pub struct MssqlConnection;

impl SimplePluginCommand for MssqlConnection {
    type Plugin = MssqlPlugin;

    fn name(&self) -> &str {
        "mssql connect"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build(SimplePluginCommand::name(self))
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

    fn usage(&self) -> &str {
        "A plugin for connecting to a MSSQL database"
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: &nu_protocol::Value,
    ) -> Result<nu_protocol::Value, nu_protocol::LabeledError> {
        let settings = ConnectionSettings::from_call(call)?;
        let wrapper = task::block_on(StreamWrapper::new(settings));
        Ok(Value::custom(Box::new(wrapper), Span::unknown()))
    }
}
