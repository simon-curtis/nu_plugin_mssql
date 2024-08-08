use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_mssql::MssqlPlugin;

fn main() {
    serve_plugin(&MssqlPlugin::new(), MsgPackSerializer)
}
