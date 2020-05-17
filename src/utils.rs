use std::time::SystemTime;

pub struct CreatedAt {
    pub seconds_since_unix: u64,
    pub nano_seconds: u32,
}

pub fn get_time() -> Result<CreatedAt, &'static str> {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH);

    let time = match time {
        Ok(t) => t,
        Err(_) => return Err("Error getting system time."),
    };

    Ok(
        CreatedAt {
            seconds_since_unix: time.as_secs(),
            nano_seconds: time.subsec_nanos(),
        }
    )
}
pub fn instant_to_seconds(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error in getting time bucket")
        .as_secs()
}