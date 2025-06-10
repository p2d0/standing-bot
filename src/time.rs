use chrono::{DateTime, Utc};

pub fn get_time_difference_from_now(timestamp: i64) -> String {
    return get_time_difference(timestamp, Utc::now().timestamp());
}

pub fn get_time_difference(timestamp: i64, end_timestamp: i64) -> String {
    if let (Some(first), Some(end)) = (DateTime::from_timestamp(timestamp,0), DateTime::from_timestamp(end_timestamp,0))
    {
        let difference = end.signed_duration_since(first);
        let minutes = difference.num_minutes();
        let seconds = difference.num_seconds() % 60;
        return format!("{minutes} минут {seconds} секунд");
    } else {
        return "".to_string();
    }
}

pub fn total_seconds_to_hms(total: i64) -> String {
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    return format!("{hours} часов {minutes} минут {seconds} секунд");
}


pub fn get_seconds_difference_from_now(timestamp: i64) -> i64 {
    get_seconds_difference(timestamp,Utc::now())
}

pub fn get_seconds_difference(timestamp: i64,now:DateTime<Utc>) -> i64 {
    if let Some(past) = DateTime::from_timestamp(timestamp,0) {
        let difference = now.signed_duration_since(past);
        return difference.num_seconds();
    } else {
        return 0;
    }
}
