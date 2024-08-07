mod mssql;
mod query;
mod connect;

use nu_protocol::LabeledError;
pub use mssql::Mssql;
pub use connect::Connect;
pub use query::Query;

use crate::data::{ConnectionArgs, ConnectionError};

fn handle_err(
    error: ConnectionError,
    args: &ConnectionArgs
) -> LabeledError {
    match error {
        ConnectionError::LoginFailed(auth_method) => {
            match auth_method {
                #[cfg(target_os = "windows")]
                tiberius::AuthMethod::Integrated => {
                    LabeledError::new("Login failed for integrated auth")
                }
                #[cfg(target_os = "windows")]
                tiberius::AuthMethod::Windows(_) => LabeledError::new("Windows auth not supported yet"),
                tiberius::AuthMethod::None => LabeledError::new("Login failed for none auth"),
                tiberius::AuthMethod::SqlServer(_) => LabeledError::new(format!(
                    "Login failed for user {:?}, password: <HIDDEN>",
                    args.user.as_ref().unwrap().as_str(),
                )),
                tiberius::AuthMethod::AADToken(_) => LabeledError::new("AADToken auth not supported"),
                #[cfg(target_os = "linux")]
                _ => todo!(),
            }
        }
        ConnectionError::UserWithoutPassword(span) => {
            LabeledError::new("Invalid credentials")
                .with_label("User specified without password", span)
        }
        ConnectionError::SetupError(error) => {
            LabeledError::new(format!(
                "Error while setting up connection: {}",
                error.to_string()
            ))
        }
        ConnectionError::ConnectionError(error) => {
            LabeledError::new(format!(
                "Error while connecting to database: {}",
                error.to_string()
            ))
        }
    }
}