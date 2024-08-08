use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};

use async_std::net::TcpStream;
use nu_protocol::{LabeledError, ShellError, Value};
use tiberius::{error::Error, AuthMethod, Client, Config, SqlBrowser};

use super::{Connection, ConnectionArgs, ConnectionError};

pub struct ConnectionPool {
    connections: Mutex<HashMap<ConnectionArgs, Connection>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
        }
    }

    fn lock(
        &self,
    ) -> Result<MutexGuard<HashMap<ConnectionArgs, Connection>>, ShellError> {
        self.connections
            .lock()
            .map_err(|e| ShellError::GenericError {
                error: format!("error acquiring pool lock: {e}"),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
    }

    pub async fn create_connection(
        &self,
        engine: &nu_plugin::EngineInterface,
        args: ConnectionArgs,
    ) -> anyhow::Result<Connection, ShellError> {
        eprintln!("Connection pool: Creating connection");

        let config = match config_from_args(&args) {
            Ok(config) => config,
            Err(e) => return Err(e.to_shell_error(&args)),
        };

        let stream = match create_stream(&args, &config).await {
            Ok(stream) => stream,
            Err(e) => return Err(ConnectionError::SetupError(e).to_shell_error(&args)),
        };
    
        let connection = match Client::connect(config, stream).await {
            Ok(client) => Connection::new(Arc::new(Mutex::new(client))),
            Err(Error::Server(e)) if e.code() == 18456 => {
                let auth = get_auth_method(&args).unwrap();
                return Err(ConnectionError::LoginFailed(auth).to_shell_error(&args))
            }
            Err(e) => return Err(ConnectionError::ConnectionError(e).to_shell_error(&args)),
        };

        let mut lock = self.lock()?;
        let _ = lock.insert(args, connection.clone());

        eprintln!("ConnectionPool: Pool has values disabling GC");
        engine.set_gc_disabled(true).map_err(LabeledError::from)?;

        drop(lock);
        Ok(connection.clone())
    }

    pub fn get(&self, args: &ConnectionArgs, increment: bool) -> Result<Option<Connection>, ShellError> {
        let mut lock = self.lock()?;
        let result = lock.get_mut(args).map(|cv| {
            if increment {
                cv.reference_count += 1;
            }
            cv.clone()
        });
        drop(lock);
        Ok(result)
    }

    pub async fn close(&self) {
        for connection in self.connections.lock().unwrap().values_mut() {
            connection.close().await;
        }
    }
}

pub async fn create_stream(
    args: &ConnectionArgs,
    config: &Config,
) -> anyhow::Result<TcpStream, tiberius::error::Error> {
    let tcp = match &args.instance {
        Some(_) => TcpStream::connect_named(config).await?,
        None => TcpStream::connect(config.get_addr()).await?,
    };

    tcp.set_nodelay(true)?;
    Ok(tcp)
}

fn config_from_args(args: &ConnectionArgs) -> anyhow::Result<Config, ConnectionError> {
    let mut config = Config::new();

    if let Some(server) = &args.server {
        config.host(server.as_str().unwrap());
    } else {
        config.host("localhost");
    }

    if let Some(database) = &args.database {
        config.database(database.as_str().unwrap());
    } else {
        config.database("master");
    }

    if let Some(instance) = &args.instance {
        config.instance_name(instance.as_str().unwrap());
    } else {
        config.port(1433)
    }

    match get_auth_method(args) {
        Ok(auth_method) => config.authentication(auth_method),
        Err(e) => return Err(e),
    }

    if args.trust_cert.is_some() {
        config.trust_cert();
    }

    Ok(config)
}

fn get_auth_method(args: &ConnectionArgs) -> anyhow::Result<AuthMethod, ConnectionError> {
    match (&args.user, &args.password) {
        (Some(Value::String { val: user, .. }), Some(Value::String { val: password, .. })) => {
            Ok(AuthMethod::sql_server(user, password))
        }
        (_, Some(Value::String { val: password, .. })) => {
            Ok(AuthMethod::sql_server("sa", password))
        }
        (Some(password), None) => Err(ConnectionError::UserWithoutPassword(password.span())),
        #[cfg(target_os = "windows")]
        _ => Ok(AuthMethod::Integrated),
        #[cfg(target_os = "linux")]
        _ => Ok(AuthMethod::None),
    }
}
