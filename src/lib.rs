mod commands;
mod data;

use async_std::task;
use commands::Mssql;
use data::ConnectionPool;
use nu_plugin::{Plugin, PluginCommand};

pub use commands::Query;

pub const DEFAULT_BUFFER_SIZE: usize = 10;

pub struct MssqlPlugin {
    pub(crate) connection_pool: ConnectionPool,
}

impl MssqlPlugin {
    pub fn new() -> Self {
        Self {
            connection_pool: ConnectionPool::new(),
        }
    }
}

impl Plugin for MssqlPlugin {
    fn version(&self) -> String {
        // This automatically uses the version of your package from Cargo.toml as the plugin version
        // sent to Nushell
        env!("CARGO_PKG_VERSION").into()
    }

    fn custom_value_dropped(
            &self,
            _engine: &nu_plugin::EngineInterface,
            _custom_value: Box<dyn nu_protocol::CustomValue>,
        ) -> Result<(), nu_protocol::LabeledError> {
        eprintln!("Dropping all the connections");
        task::block_on(self.connection_pool.close());
        Ok(())
    }

    fn commands(&self) -> Vec<Box<dyn PluginCommand<Plugin = Self>>> {
        vec![Box::new(Mssql), Box::new(Query)]
    }
}