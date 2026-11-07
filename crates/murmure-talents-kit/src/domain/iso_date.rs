//! ISO 8601 dates of the index (`2026-11-02T10:00:00Z` or with an offset) as Unix seconds.

/// `2026-10-31T19:28:47+01:00` or `…Z` as seconds since 1970; anything else is `None`.
pub fn parse_iso(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if !text.is_ascii() || bytes.len() < 20 {
        return None;
    }
    let number = |from: usize, to: usize| -> Option<i64> {
        let part = text.get(from..to)?;
        part.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| part.parse().ok())?
    };
    if [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')]
        .iter()
        .any(|(at, expected)| bytes[*at] != *expected)
    {
        return None;
    }
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let offset = match &text[19..] {
        "Z" => 0,
        zone if zone.len() == 6 && (zone.starts_with('+') || zone.starts_with('-')) => {
            if zone.as_bytes()[3] != b':' {
                return None;
            }
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            sign * (number(20, 22)? * 3_600 + number(23, 25)? * 60)
        }
        _ => return None,
    };
    // Days from a civil date (Howard Hinnant's algorithm).
    let year_shifted = year - i64::from(month <= 2);
    let era = year_shifted.div_euclid(400);
    let year_of_era = year_shifted.rem_euclid(400);
    let month_shifted = (month + 9) % 12;
    let day_of_year = (153 * month_shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second - offset)
}
