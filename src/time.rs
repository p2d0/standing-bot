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

pub fn get_total_string(total: i64) -> String {
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    return format!("{hours} часов {minutes} минут {seconds} секунд");
}

pub fn get_seconds_difference(timestamp: i64) -> i64 {
    let now = Utc::now();
    if let Some(past) = DateTime::from_timestamp(timestamp,0) {
        let difference = now.signed_duration_since(past);
        return difference.num_seconds();
    } else {
        return 0;
    }
}
