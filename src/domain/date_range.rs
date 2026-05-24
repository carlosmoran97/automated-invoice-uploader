use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRange {
    start: Date,
    end: Date,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Date {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateRangeError {
    InvalidInitialDate,
    InvalidFinalDate,
    InitialAfterFinal,
}

impl DateRange {
    pub fn parse(initial_date: &str, final_date: &str) -> Result<Self, DateRangeError> {
        let start = Date::parse(initial_date).ok_or(DateRangeError::InvalidInitialDate)?;
        let end = Date::parse(final_date).ok_or(DateRangeError::InvalidFinalDate)?;

        if start > end {
            return Err(DateRangeError::InitialAfterFinal);
        }

        Ok(Self { start, end })
    }

    pub fn initial_date(&self) -> String {
        self.start.to_string()
    }

    pub fn final_date(&self) -> String {
        self.end.to_string()
    }

    pub fn gmail_after_date(&self) -> String {
        self.start.to_gmail_string()
    }

    pub fn gmail_before_date(&self) -> String {
        self.end.next_day().to_gmail_string()
    }
}

impl Date {
    fn parse(value: &str) -> Option<Self> {
        let bytes = value.as_bytes();
        if value.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
            return None;
        }

        let year = value[0..4].parse::<u16>().ok()?;
        let month = value[5..7].parse::<u8>().ok()?;
        let day = value[8..10].parse::<u8>().ok()?;
        let max_day = days_in_month(year, month)?;

        if year == 0 || day == 0 || day > max_day {
            return None;
        }

        Some(Self { year, month, day })
    }

    fn next_day(self) -> Self {
        let max_day = days_in_month(self.year, self.month).unwrap_or(31);
        if self.day < max_day {
            return Self {
                day: self.day + 1,
                ..self
            };
        }

        if self.month < 12 {
            return Self {
                month: self.month + 1,
                day: 1,
                ..self
            };
        }

        Self {
            year: self.year + 1,
            month: 1,
            day: 1,
        }
    }

    fn to_gmail_string(self) -> String {
        format!("{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

impl fmt::Display for DateRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInitialDate => {
                write!(formatter, "Initial date must use a valid YYYY-MM-DD value.")
            }
            Self::InvalidFinalDate => {
                write!(formatter, "Final date must use a valid YYYY-MM-DD value.")
            }
            Self::InitialAfterFinal => {
                write!(
                    formatter,
                    "Initial date must be before or equal to final date."
                )
            }
        }
    }
}

impl std::error::Error for DateRangeError {}

fn days_in_month(year: u16, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

fn is_leap_year(year: u16) -> bool {
    let year = year as u32;
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_date_range() {
        let range = DateRange::parse("2026-05-01", "2026-05-31").unwrap();

        assert_eq!(range.initial_date(), "2026-05-01");
        assert_eq!(range.final_date(), "2026-05-31");
        assert_eq!(range.gmail_after_date(), "2026/05/01");
        assert_eq!(range.gmail_before_date(), "2026/06/01");
    }

    #[test]
    fn rejects_invalid_dates() {
        assert_eq!(
            DateRange::parse("2026-02-30", "2026-03-01"),
            Err(DateRangeError::InvalidInitialDate)
        );
        assert_eq!(
            DateRange::parse("2026-02-01", "2026-02-30"),
            Err(DateRangeError::InvalidFinalDate)
        );
    }

    #[test]
    fn supports_leap_years() {
        let range = DateRange::parse("2024-02-01", "2024-02-29").unwrap();

        assert_eq!(range.gmail_before_date(), "2024/03/01");
    }

    #[test]
    fn rejects_reversed_range() {
        assert_eq!(
            DateRange::parse("2026-05-31", "2026-05-01"),
            Err(DateRangeError::InitialAfterFinal)
        );
    }
}
