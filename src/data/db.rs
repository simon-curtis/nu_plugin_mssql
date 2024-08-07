use async_std::channel::Receiver;
use nu_protocol::{LabeledError, Span, Value};
use tiberius::{
    time::{
        chrono::{DateTime, FixedOffset, NaiveDateTime},
        Time,
    },
    ColumnData, FromSql,
};

pub fn parse_value(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
    match data {
        ColumnData::Binary(Some(val)) => Ok(Value::binary(val.as_ref(), Span::unknown())),
        ColumnData::Binary(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Bit(Some(val)) => Ok(Value::bool(*val, Span::unknown())),
        ColumnData::Bit(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::String(Some(val)) => Ok(Value::string(val.as_ref(), Span::unknown())),
        ColumnData::String(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::U8(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::U8(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::I16(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::I16(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::I32(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::I32(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::I64(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::I64(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::F32(Some(val)) => Ok(Value::float(*val as f64, Span::unknown())),
        ColumnData::F32(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::F64(Some(val)) => Ok(Value::float(*val as f64, Span::unknown())),
        ColumnData::F64(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Date(Some(_)) => parse_datetime(data),
        ColumnData::Date(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Time(Some(time)) => parse_time(time),
        ColumnData::Time(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::DateTime(Some(_)) => parse_datetime(data),
        ColumnData::DateTime(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::DateTime2(Some(_)) => parse_datetime(data),
        ColumnData::DateTime2(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::DateTimeOffset(Some(_)) => parse_datetime(data),
        ColumnData::DateTimeOffset(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::SmallDateTime(Some(_)) => parse_datetime(data),
        ColumnData::SmallDateTime(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Guid(Some(guid)) => Ok(Value::string(guid.to_string(), Span::unknown())),
        ColumnData::Guid(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Numeric(Some(numeric)) => {
            Ok(Value::float(numeric.value() as f64, Span::unknown()))
        }
        ColumnData::Numeric(None) => Ok(Value::nothing(Span::unknown())),
        ColumnData::Xml(Some(xml)) => Ok(Value::string(xml.to_string(), Span::unknown())),
        ColumnData::Xml(None) => Ok(Value::nothing(Span::unknown())),
    }
}

fn parse_time(time: &Time) -> anyhow::Result<Value, LabeledError> {
    // Number of 10^-n second increments since midnight, where n is defined in scale.
    let increments = time.increments();

    // The accuracy of the time is defined in scale.
    let scale = time.scale() as u32;

    // Calculate duration in nanoseconds, we use 9 because there are 10^9 nanoseconds in a second
    let duration = increments * 10u64.pow(9 - scale);

    Ok(Value::duration(duration as i64, Span::unknown()))
}

fn parse_datetime(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
    match NaiveDateTime::from_sql(data) {
        Ok(naive) => match naive {
            Some(naive) => match FixedOffset::east_opt(0) {
                Some(offset) => {
                    let date_time =
                        DateTime::<FixedOffset>::from_naive_utc_and_offset(naive, offset);
                    Ok(Value::date(date_time, Span::unknown()))
                }
                None => Err(LabeledError::new("Failed to parse datetime")
                    .with_label("Invalid datetime", Span::unknown())),
            },
            None => Err(LabeledError::new("Failed to parse datetime")
                .with_label("Invalid datetime", Span::unknown())),
        },
        Err(e) => Err(LabeledError::new("Failed to parse datetime")
            .with_label(e.to_string(), Span::unknown())),
    }
}

pub struct TableIterator {
    receiver: Receiver<Value>,
}

impl TableIterator {
    pub fn new(receiver: Receiver<Value>) -> Self {
        Self { receiver }
    }
}

impl Iterator for TableIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv_blocking() {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }
}
