use chrono::{DateTime, Utc};

pub fn get_time_difference(timestamp: i64) -> String {
    let now = Utc::now();
    if let Some(past) = DateTime::from_timestamp(timestamp,0) {
        let difference = now.signed_duration_since(past);
        let minutes = difference.num_minutes();
        let seconds = difference.num_seconds() % 60;
        return format!("{minutes} минут {seconds} секунд");
    } else {
        return "".to_string();
    }
}
