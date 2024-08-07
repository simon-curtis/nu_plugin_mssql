use async_std::net::TcpStream;
use nu_protocol::{Span, Value};
use tiberius::{error::Error, AuthMethod, Client, Config, SqlBrowser};

use super::ConnectionArgs;


#[derive(Debug)]
pub enum ConnectionError {
    UserWithoutPassword(Span),
    LoginFailed(AuthMethod),
    SetupError(tiberius::error::Error),
    ConnectionError(tiberius::error::Error),
}

pub async fn create_client(args: &ConnectionArgs) -> anyhow::Result<Client<TcpStream>, ConnectionError> {
    let config = config_from_args(args)?;
    let stream = match create_stream(args, &config).await {
        Ok(stream) => stream,
        Err(e) => return Err(ConnectionError::SetupError(e)),
    };

    match Client::connect(config, stream).await {
        Ok(client) => Ok(client),
        Err(Error::Server(e)) if e.code() == 18456 => {
            let auth = get_auth_method(&args).unwrap();
            Err(ConnectionError::LoginFailed(auth))
        }
        Err(e) => Err(ConnectionError::ConnectionError(e)),
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
