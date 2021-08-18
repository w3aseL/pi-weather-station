use serde::{ Serialize, Deserialize };
use chrono::{ DateTime, Utc, Date, Local };
use postgres::{ Client };

use super::{ DatabaseType };

#[derive(Debug, Serialize, Deserialize)]
pub struct Rain {
    #[serde(with = "dt_format")]
    timestamp: DateTime<Utc>,
    count: u32
}

impl Rain {
    pub fn new(timestamp: DateTime<Utc>, count: u32) -> Self {
        Self {
            timestamp,
            count
        }
    }
}

impl DatabaseType for Rain {
    fn create_table(client: &mut Client) {
        client.batch_execute("
            CREATE TABLE IF NOT EXISTS Rain (
                id              SERIAL PRIMARY KEY,
                rain_counter    INTEGER DEFAULT 0 NOT NULL,
                timestamp       TIMESTAMP NOT NULL
            )
        ").expect("Failed to create table!");
    }

    fn insert(&self, client: &mut Client) {
        let timestamp = format!("{}", self.timestamp.format("%Y-%m-%d %H:%M:%S"));

        client.execute("INSERT INTO Rain (rain_counter, timestamp) VALUES ($1, $2)",
             &[&self.count, &timestamp]).expect("Failed to insert rain data!");
    }

    fn insert_many<T: DatabaseType>(data: Vec<T>, client: &mut Client) {
        
    }

    fn find_all<T: DatabaseType>() -> Vec<T> {
        Vec::new()
    }
}

// https://serde.rs/custom-date-format.html
mod dt_format {
    use chrono::{DateTime, Utc, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(
        date: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

mod date_format {
    use chrono::{TimeZone, Date, Local, NaiveDate};
    use serde::{self, Deserialize, Serializer, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(
        date: &Date<Local>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Date<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let date = NaiveDate::parse_from_str(&s, FORMAT).unwrap();
        Ok(Local.from_local_date(&date).unwrap())
    }
}