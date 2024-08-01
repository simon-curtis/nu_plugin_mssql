use nu_plugin::{serve_plugin, JsonSerializer};
use nu_plugin::{Plugin, PluginCommand};
use query::MssqlPluginQuery;

mod db;
mod query;

pub struct MssqlPlugin;

impl Plugin for MssqlPlugin {
    fn version(&self) -> String {
        "0.0.1".to_string()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(MssqlPluginQuery)]
    }
}

fn main() {
    serve_plugin(&MssqlPlugin, JsonSerializer)
}
