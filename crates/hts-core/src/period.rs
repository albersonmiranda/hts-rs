//! Time period types for time series indexing.

use crate::error::{HtsError, Result};
use chrono::NaiveDate;
use std::fmt;
use std::str::FromStr;

/// A time period used as the index for time series data.
///
/// Supports various frequencies including Annual, Quarterly, Monthly,
/// Weekly, and Daily.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// Annual period (e.g., "1998").
    Annual(i32),
    /// Quarterly period (e.g., "1998 Q1").
    Quarterly(i32, u8),
    /// Monthly period (e.g., "1998 M01").
    Monthly(i32, u8),
    /// Weekly period (e.g., "1998 W01").
    Weekly(i32, u8),
    /// Daily period (e.g., "1998-01-01").
    Daily(NaiveDate),
}

impl Period {
    /// Parses a string into a `Period`.
    ///
    /// Auto-detects the format:
    /// - "YYYY" -> Annual
    /// - "YYYY QN" -> Quarterly
    /// - "YYYY MN" -> Monthly
    /// - "YYYY WN" -> Weekly
    /// - "YYYY-MM-DD" -> Daily
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        // Daily: YYYY-MM-DD
        if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Ok(Self::Daily(date));
        }

        let parts: Vec<&str> = s.split_whitespace().collect();

        // Annual: YYYY
        if parts.len() == 1 {
            if let Ok(year) = parts[0].parse::<i32>() {
                return Ok(Self::Annual(year));
            }
        }

        if parts.len() != 2 {
            return Err(HtsError::InvalidPeriod(format!(
                "Unknown period format: '{s}'"
            )));
        }

        let year: i32 = parts[0]
            .parse()
            .map_err(|_| HtsError::InvalidPeriod(format!("Invalid year in '{s}'")))?;

        let suffix = parts[1];
        if suffix.len() < 2 {
            return Err(HtsError::InvalidPeriod(format!(
                "Invalid period suffix: '{s}'"
            )));
        }

        let indicator = suffix.chars().next().unwrap().to_ascii_uppercase();
        let value_str = &suffix[1..];
        let value: u32 = value_str
            .parse()
            .map_err(|_| HtsError::InvalidPeriod(format!("Invalid number in period: '{s}'")))?;

        match indicator {
            'Q' => {
                if !(1..=4).contains(&value) {
                    return Err(HtsError::InvalidPeriod(format!(
                        "Quarter must be 1-4, got {value}"
                    )));
                }
                Ok(Self::Quarterly(year, value as u8))
            }
            'M' => {
                if !(1..=12).contains(&value) {
                    return Err(HtsError::InvalidPeriod(format!(
                        "Month must be 1-12, got {value}"
                    )));
                }
                Ok(Self::Monthly(year, value as u8))
            }
            'W' => {
                if !(1..=53).contains(&value) {
                    return Err(HtsError::InvalidPeriod(format!(
                        "Week must be 1-53, got {value}"
                    )));
                }
                Ok(Self::Weekly(year, value as u8))
            }
            _ => Err(HtsError::InvalidPeriod(format!(
                "Unknown period type '{indicator}' in '{s}'"
            ))),
        }
    }

    /// Returns the start date of the period.
    pub fn to_naive_date(self) -> NaiveDate {
        match self {
            Self::Annual(y) => NaiveDate::from_ymd_opt(y, 1, 1).expect("Valid annual date"),
            Self::Quarterly(y, q) => {
                let month = match q {
                    1 => 1,
                    2 => 4,
                    3 => 7,
                    4 => 10,
                    _ => unreachable!(),
                };
                NaiveDate::from_ymd_opt(y, month, 1).expect("Valid quarterly date")
            }
            Self::Monthly(y, m) => {
                NaiveDate::from_ymd_opt(y, m as u32, 1).expect("Valid monthly date")
            }
            Self::Weekly(y, w) => {
                // ISO week date approximation (first day of week)
                NaiveDate::from_isoywd_opt(y, w as u32, chrono::Weekday::Mon)
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(y, 1, 1).unwrap())
            }
            Self::Daily(d) => d,
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Annual(y) => write!(f, "{}", y),
            Self::Quarterly(y, q) => write!(f, "{} Q{}", y, q),
            Self::Monthly(y, m) => write!(f, "{} M{:02}", y, m),
            Self::Weekly(y, w) => write!(f, "{} W{:02}", y, w),
            Self::Daily(d) => write!(f, "{}", d.format("%Y-%m-%d")),
        }
    }
}

impl FromStr for Period {
    type Err = HtsError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_annual() {
        assert_eq!(Period::parse("1998").unwrap(), Period::Annual(1998));
    }

    #[test]
    fn test_parse_quarterly() {
        assert_eq!(
            Period::parse("1998 Q1").unwrap(),
            Period::Quarterly(1998, 1)
        );
        assert_eq!(
            Period::parse("2024 q4").unwrap(),
            Period::Quarterly(2024, 4)
        );
    }

    #[test]
    fn test_parse_monthly() {
        assert_eq!(Period::parse("1998 M01").unwrap(), Period::Monthly(1998, 1));
        assert_eq!(
            Period::parse("2024 m12").unwrap(),
            Period::Monthly(2024, 12)
        );
    }

    #[test]
    fn test_parse_weekly() {
        assert_eq!(Period::parse("1998 W01").unwrap(), Period::Weekly(1998, 1));
    }

    #[test]
    fn test_parse_daily() {
        assert_eq!(
            Period::parse("1998-01-01").unwrap(),
            Period::Daily(NaiveDate::from_ymd_opt(1998, 1, 1).unwrap())
        );
    }

    #[test]
    fn test_ordering() {
        let p1 = Period::Quarterly(1998, 1);
        let p2 = Period::Quarterly(1998, 2);
        assert!(p1 < p2);

        let m1 = Period::Monthly(1998, 1);
        let m2 = Period::Monthly(1998, 2);
        assert!(m1 < m2);
    }

    #[test]
    fn test_display() {
        assert_eq!(Period::Quarterly(1998, 1).to_string(), "1998 Q1");
        assert_eq!(Period::Monthly(1998, 1).to_string(), "1998 M01");
        assert_eq!(Period::Annual(1998).to_string(), "1998");
    }
}
