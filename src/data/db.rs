use async_std::{
    channel::{Receiver, Sender},
    stream::StreamExt,
};
use nu_protocol::{LabeledError, Record, Span, Value};
use tiberius::{
    time::chrono::{DateTime, FixedOffset, NaiveDateTime},
    ColumnData, FromSql, Query,
};

use crate::data::connection::ConnectionSettings;

pub fn parse_value(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
    match data {
        ColumnData::Binary(Some(val)) => Ok(Value::binary(val.as_ref(), Span::unknown())),
        ColumnData::String(Some(val)) => Ok(Value::string(val.as_ref(), Span::unknown())),
        ColumnData::I32(Some(val)) => Ok(Value::int(*val as i64, Span::unknown())),
        ColumnData::F32(Some(val)) => Ok(Value::float(*val as f64, Span::unknown())),
        ColumnData::DateTime2(Some(_)) => parse_date(data),
        other => Err(LabeledError::new(format!(
            "Failed to parse value: {:?}",
            other
        ))),
    }
}

fn parse_date(data: &ColumnData<'static>) -> anyhow::Result<Value, LabeledError> {
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
    receiver: Receiver<Record>,
}

impl TableIterator {
    pub fn new(receiver: Receiver<Record>) -> Self {
        Self { receiver }
    }
}

impl Iterator for TableIterator {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.receiver.recv_blocking() {
            Ok(record) => Some(Value::record(record, Span::unknown())),
            Err(_) => None,
        }
    }
}

pub async fn run_query(query: String, settings: ConnectionSettings, sender: Sender<Record>) {
    let mut client = match settings.create_client(&settings).await {
        Ok(client) => client,
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    };

    let select = Query::new(query);
    let stream = match select.query(&mut client).await {
        Ok(stream) => stream,
        Err(e) => {
            panic!("Error: {}", e);
        }
    };

    let mut row_stream = stream.into_row_stream();
    while let Some(row) = row_stream.next().await {
        match row {
            Ok(row) => {
                let mut record = Record::new();

                for (col, cell) in row.cells() {
                    match parse_value(cell) {
                        Ok(value) => {
                            record.insert(col.name(), value);
                        }
                        Err(e) => {
                            panic!("Error: {:?}", e);
                        }
                    }
                }

                if let Err(e) = sender.send(record).await {
                    if sender.is_closed() {
                        return;
                    }
                    panic!("Error: {:?}", e);
                }
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
        }
    }
}
