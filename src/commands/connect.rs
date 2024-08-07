use crate::data::{create_client, MssqlClient, ConnectionArgs};
use async_std::task;
use nu_plugin::{PluginCommand, SimplePluginCommand};
use nu_protocol::{
    Signature, SyntaxShape, Value,
};

use crate::MssqlPlugin;

use super::handle_err;

pub struct Connect;

impl SimplePluginCommand for Connect {
    type Plugin = MssqlPlugin;

    fn name(&self) -> &str {
        "mssql connect"
    }

    fn usage(&self) -> &str {
        "Connect to a MSSQL database"
    }

    fn signature(&self) -> Signature {
        Signature::build(PluginCommand::name(self))
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
                "trust-cert",
                SyntaxShape::Boolean,
                "Trust the server certificate",
                Some('t'),
            )
            .category(nu_protocol::Category::Database)
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        _engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: &nu_protocol::Value,
    ) -> Result<Value, nu_protocol::LabeledError> {
        task::block_on(async move {
            let args = ConnectionArgs::from_call(call)?;
            let client = match create_client(&args).await {
                Ok(client) => client,
                Err(e) => return Err(handle_err(e, &args))
            };

            let wrapper = MssqlClient::new(args, client);
            Ok(Value::custom(Box::new(wrapper), call.head))
        })
    }
}