mod commands;
mod data;

use commands::Mssql;
use nu_plugin::{Plugin, PluginCommand};

pub use commands::Query;

pub const DEFAULT_BUFFER_SIZE: usize = 10;

pub struct MssqlPlugin;

impl Plugin for MssqlPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Mssql), Box::new(Query)]
    }
}
