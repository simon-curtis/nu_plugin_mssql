use nu_plugin::{EngineInterface, EvaluatedCall, SimplePluginCommand};
use nu_protocol::{Category, LabeledError, Signature, Value};

use crate::MssqlPlugin;

pub struct Mssql;

impl SimplePluginCommand for Mssql {
    type Plugin = MssqlPlugin;

    fn name(&self) -> &str {
        "mssql"
    }

    fn signature(&self) -> Signature {
        Signature::build(self.name()).category(Category::Database)
    }

    fn usage(&self) -> &str {
        "Return information about the dt set of commands"
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["mssql"]
    }

    fn run(
        &self,
        _plugin: &MssqlPlugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: &Value,
    ) -> Result<Value, LabeledError> {
        Ok(Value::string(engine.get_help()?, call.head))
    }
}
