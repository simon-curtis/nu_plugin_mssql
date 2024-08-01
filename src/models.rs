use std::cmp::Ordering;

use async_std::net::TcpStream;
use nu_protocol::{ast::Operator, CustomValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};

use crate::data::connection::ConnectionSettings;

#[derive(Debug, Clone)]
pub struct StreamWrapper {
    pub stream: Option<Box<TcpStream>>,
    pub settings: ConnectionSettings,
}

impl StreamWrapper {
    pub async fn new(settings: ConnectionSettings) -> Self {
        let config = settings.to_config().unwrap();
        Self {
            stream: match settings.create_stream(&config).await {
                Ok(stream) => Some(Box::new(stream)),
                Err(e) => panic!("Error: {:?}", e),
            },
            settings,
        }
    }
}

impl Serialize for StreamWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Here we are just going to serialize the config
        self.settings.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StreamWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Here we are just going to deserialize the config
        let config = ConnectionSettings::deserialize(deserializer)?;
        Ok(StreamWrapper {
            settings: config,
            stream: None,
        })
    }
}

impl CustomValue for StreamWrapper {
    #[doc = " Custom `Clone` implementation"]
    #[doc = ""]
    #[doc = " This can reemit a `Value::CustomValue(Self, span)` or materialize another representation"]
    #[doc = " if necessary."]
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    #[doc = " The friendly type name to show for the custom value, e.g. in `describe` and in error"]
    #[doc = " messages. This does not have to be the same as the name of the struct or enum, but"]
    #[doc = " conventionally often is."]
    fn type_name(&self) -> String {
        "Stream".into()
    }

    #[doc = " Converts the custom value to a base nushell value."]
    #[doc = ""]
    #[doc = " This imposes the requirement that you can represent the custom value in some form using the"]
    #[doc = " Value representations that already exist in nushell"]
    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        Ok(Value::custom(Box::new(self.clone()), span))
    }

    #[doc = " Any representation used to downcast object to its original type"]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    #[doc = " Any representation used to downcast object to its original type (mutable reference)"]
    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[doc(hidden)]
    fn typetag_name(&self) -> &'static str {
        "Stream"
    }

    #[doc(hidden)]
    fn typetag_deserialize(&self) {}

    #[doc = " Follow cell path by numeric index (e.g. rows)"]
    fn follow_path_int(
        &self,
        self_span: Span,
        index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, index);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    #[doc = " Follow cell path by string key (e.g. columns)"]
    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, column_name);
        Err(ShellError::IncompatiblePathAccess {
            type_name: self.type_name(),
            span: path_span,
        })
    }

    #[doc = " ordering with other value (see [`std::cmp::PartialOrd`])"]
    fn partial_cmp(&self, _other: &Value) -> Option<Ordering> {
        None
    }

    #[doc = " Definition of an operation between the object that implements the trait"]
    #[doc = " and another Value."]
    #[doc = ""]
    #[doc = " The Operator enum is used to indicate the expected operation."]
    #[doc = ""]
    #[doc = " Default impl raises [`ShellError::UnsupportedOperator`]."]
    fn operation(
        &self,
        lhs_span: Span,
        operator: Operator,
        op: Span,
        right: &Value,
    ) -> Result<Value, ShellError> {
        let _ = (lhs_span, right);
        Err(ShellError::UnsupportedOperator { operator, span: op })
    }

    #[doc = " For custom values in plugins: return `true` here if you would like to be notified when all"]
    #[doc = " copies of this custom value are dropped in the engine."]
    #[doc = ""]
    #[doc = " The notification will take place via `custom_value_dropped()` on the plugin type."]
    #[doc = ""]
    #[doc = " The default is `false`."]
    fn notify_plugin_on_drop(&self) -> bool {
        true
    }
}
