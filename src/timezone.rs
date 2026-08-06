use chrono::{DateTime, Duration, LocalResult, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use std::str::FromStr;

pub fn parse(name: &str) -> Option<Tz> {
    Tz::from_str(name).ok()
}

pub fn next_five_am(now: DateTime<Utc>, timezone: Tz) -> Option<DateTime<Utc>> {
    let local_now = now.with_timezone(&timezone);
    let mut date = local_now.date_naive();
    if local_now.time() >= NaiveTime::from_hms_opt(5, 0, 0).expect("valid time") {
        date += Duration::days(1);
    }

    let local = date.and_hms_opt(5, 0, 0).expect("valid time");
    match timezone.from_local_datetime(&local) {
        LocalResult::Single(value) | LocalResult::Ambiguous(value, _) => {
            Some(value.with_timezone(&Utc))
        }
        LocalResult::None => {
            let mut candidate = local;
            for _ in 0..(48 * 60) {
                candidate += Duration::minutes(1);
                match timezone.from_local_datetime(&candidate) {
                    LocalResult::Single(value) | LocalResult::Ambiguous(value, _) => {
                        return Some(value.with_timezone(&Utc));
                    }
                    LocalResult::None => {}
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{next_five_am, parse};
    use chrono::{TimeZone, Utc};

    #[test]
    fn accepts_iana_names_but_not_offsets() {
        assert!(parse("Asia/Bangkok").is_some());
        assert!(parse("America/New_York").is_some());
        assert!(parse("UTC+7").is_none());
        assert!(parse("Not/AZone").is_none());
    }

    #[test]
    fn next_boundary_crosses_dst() {
        let now = Utc.with_ymd_and_hms(2025, 3, 9, 6, 0, 0).unwrap();
        let next = next_five_am(now, parse("America/New_York").unwrap()).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 3, 9, 9, 0, 0).unwrap());
    }

    #[test]
    fn next_boundary_handles_a_skipped_day() {
        let now = Utc.with_ymd_and_hms(2011, 12, 29, 18, 0, 0).unwrap();
        let next = next_five_am(now, parse("Pacific/Apia").unwrap()).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2011, 12, 30, 10, 0, 0).unwrap());
    }
}
