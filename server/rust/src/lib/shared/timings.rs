use std::time::Duration;

pub fn update() -> Duration {
    Duration::from_millis(500)
}

pub fn expiration() -> Duration {
    update() * 10
}

pub fn stale() -> Duration {
    update() * 4
}
