use chrono::{DateTime, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// The timezone used to interpret a wall-clock time and display an instant.
#[derive(Clone, Copy, Debug)]
pub enum Zone {
    Local,
    Named(Tz),
    Fixed(FixedOffset),
}

impl Zone {
    pub fn at(self, instant: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self {
            Self::Local => instant.with_timezone(&Local).fixed_offset(),
            Self::Named(zone) => instant.with_timezone(&zone).fixed_offset(),
            Self::Fixed(offset) => instant.with_timezone(&offset),
        }
    }

    pub fn resolve(self, datetime: NaiveDateTime) -> Result<DateTime<Utc>, String> {
        let result = match self {
            Self::Local => Local
                .from_local_datetime(&datetime)
                .map(|date| date.to_utc()),
            Self::Named(zone) => zone
                .from_local_datetime(&datetime)
                .map(|date| date.to_utc()),
            Self::Fixed(offset) => offset
                .from_local_datetime(&datetime)
                .map(|date| date.to_utc()),
        };
        match result {
            LocalResult::Single(date) => Ok(date),
            LocalResult::Ambiguous(_, _) => Err("ambiguous local datetime during a timezone transition; supply an explicit UTC offset (for example -04:00 or -05:00)".into()),
            LocalResult::None => Err("local datetime does not exist in this timezone or cannot be resolved; check the daylight-saving transition or supply an explicit UTC offset".into()),
        }
    }

    pub fn label(self, instant: DateTime<Utc>) -> String {
        let offset = self.at(instant).offset().to_string();
        match self {
            Self::Local => format!("Local (UTC{offset})"),
            Self::Named(zone) => format!("{zone} (UTC{offset})"),
            Self::Fixed(_) if offset == "+00:00" => "UTC".into(),
            Self::Fixed(_) => format!("UTC{offset}"),
        }
    }
}

/// Recognize a trailing IANA timezone, UTC, or GMT. Short regional abbreviations
/// are not guessed: for example CST can mean several different offsets.
pub(crate) fn split_named(input: &str) -> Result<(&str, Option<Zone>), String> {
    let Some((date, name)) = input.rsplit_once(char::is_whitespace) else {
        return Ok((input, None));
    };
    if name == "UTC" || name == "GMT" || name.contains('/') {
        let zone = name.parse::<Tz>().map_err(|_| {
            format!("unknown timezone {name:?}; use an IANA name such as Asia/Singapore")
        })?;
        return Ok((date.trim_end(), Some(Zone::Named(zone))));
    }
    Ok((input, None))
}
