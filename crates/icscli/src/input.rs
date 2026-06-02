//! User-input parsing helpers shared between the presentation and
//! application layers. Lives outside both so neither has to depend on
//! the other (avoids a layer violation when use cases reuse CLI parsing).

use chrono::NaiveDate;

/// Parse a date in `YYYY-MM-DD` or `YYYY/M/D` form.
pub fn parse_date(s: &str) -> Result<NaiveDate, String> {
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d);
    }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() == 3 {
        if let (Ok(y), Ok(m), Ok(d)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            if let Some(date) = NaiveDate::from_ymd_opt(y, m, d) {
                return Ok(date);
            }
        }
    }
    Err(format!(
        "invalid date '{s}' (expected YYYY-MM-DD or YYYY/M/D)"
    ))
}
