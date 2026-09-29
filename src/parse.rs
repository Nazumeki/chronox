use crate::timezone::{self, Zone};
use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, NaiveDateTime, Timelike, Utc};
use clap::ValueEnum;

const NANOS_PER_SECOND: i128 = 1_000_000_000;

// A recognizer returns None when it does not match, and an error when it
// recognizes a date that cannot be represented. Order resolves overlapping syntax.
type Recognition = Option<Result<Parsed, String>>;
const RECOGNIZERS: &[fn(&str, Zone) -> Recognition] = &[rfc3339, datetime];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputKind {
    Timestamp,
    Datetime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Unit {
    #[value(alias = "s")]
    Seconds,
    #[value(alias = "ms")]
    Milliseconds,
    #[value(alias = "us")]
    Microseconds,
    #[value(alias = "ns")]
    Nanoseconds,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Seconds => "Unix timestamp (s)",
            Self::Milliseconds => "Unix timestamp (ms)",
            Self::Microseconds => "Unix timestamp (us)",
            Self::Nanoseconds => "Unix timestamp (ns)",
        }
    }

    fn scale(self) -> i128 {
        match self {
            Self::Seconds => NANOS_PER_SECOND,
            Self::Milliseconds => 1_000_000,
            Self::Microseconds => 1_000,
            Self::Nanoseconds => 1,
        }
    }
}

#[derive(Debug)]
pub struct Parsed {
    pub datetime: DateTime<Utc>,
    pub format: &'static str,
    pub kind: InputKind,
    pub timezone: Zone,
}

impl Parsed {
    pub fn nanos(&self) -> i128 {
        i128::from(self.datetime.timestamp()) * NANOS_PER_SECOND
            + i128::from(self.datetime.timestamp_subsec_nanos())
    }
}

/// Select a display timezone and use it for inputs without their own timezone.
/// Explicit input timezones always determine the instant before display conversion.
pub fn parse(input: &str, unit: Option<Unit>, target_zone: Option<Zone>) -> Result<Parsed, String> {
    let mut parsed = recognize(input, unit, target_zone.unwrap_or(Zone::Local))?;
    if let Some(zone) = target_zone {
        parsed.timezone = zone;
    }
    Ok(parsed)
}

fn recognize(input: &str, unit: Option<Unit>, default_zone: Zone) -> Result<Parsed, String> {
    let input = input.trim();
    if input.is_empty() {
        return Err("provide a timestamp or date, as an argument or through standard input".into());
    }

    let unsigned = input.strip_prefix(['-', '+']).unwrap_or(input);
    if !unsigned.is_empty()
        && unsigned
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return parse_timestamp(input, unsigned, unit);
    }
    if unit.is_some() {
        return Err("--unit applies only to numeric Unix timestamps".into());
    }
    if input
        .split('.')
        .skip(1)
        .any(|part| part.bytes().take_while(u8::is_ascii_digit).count() > 9)
    {
        return Err("datetime fractions support at most 9 digits (nanosecond precision)".into());
    }

    // Parse complete header grammars first: GMT and legacy email zone names
    // are meaningful parts of those grammars.
    if let Some(result) = headers(input, Zone::Local) {
        return result;
    }
    let (input, named_zone) = timezone::split_named(input)?;
    let zone = named_zone.unwrap_or(default_zone);
    for recognize in RECOGNIZERS {
        if let Some(result) = recognize(input, zone) {
            let result = result.and_then(|mut parsed| {
                if let Some(zone) = named_zone {
                    if let Zone::Fixed(offset) = parsed.timezone
                        && zone.at(parsed.datetime).offset() != &offset
                    {
                        return Err("input UTC offset conflicts with its named timezone".into());
                    }
                    parsed.timezone = zone;
                }
                Ok(parsed)
            });
            return result;
        }
    }
    Err("unrecognized or invalid date; expected a Unix timestamp, ISO datetime, readable datetime, or HTTP / email date. Try --help for examples".into())
}

fn rfc3339(input: &str, _: Zone) -> Recognition {
    DateTime::parse_from_rfc3339(input)
        .ok()
        .map(|date| explicit(date, "ISO datetime"))
}

fn headers(input: &str, _: Zone) -> Recognition {
    // HTTP's preferred IMF-fixdate must be distinguished from email dates.
    if let Ok(date) = NaiveDateTime::parse_from_str(input, "%a, %d %b %Y %H:%M:%S GMT") {
        return Some(explicit(date.and_utc().fixed_offset(), "HTTP date"));
    }
    if let Ok(date) = DateTime::parse_from_rfc2822(input) {
        return Some(explicit(date, "Email date"));
    }
    for format in ["%A, %d-%b-%y %H:%M:%S GMT", "%a %b %e %H:%M:%S %Y"] {
        if let Ok(date) = NaiveDateTime::parse_from_str(input, format) {
            return Some(explicit(
                date.and_utc().fixed_offset(),
                "HTTP date (obsolete)",
            ));
        }
    }
    None
}

fn datetime(input: &str, zone: Zone) -> Recognition {
    // ISO calendar, ordinal, and week dates; SQL-style space-separated dates.
    for (base, label) in [
        ("%Y-%m-%dT%H:%M:%S%.f", "ISO datetime"),
        ("%Y-%m-%dT%H:%M", "ISO datetime"),
        ("%Y%m%dT%H%M%S%.f", "ISO datetime (compact)"),
        ("%Y-%jT%H:%M:%S%.f", "ISO datetime (day of year)"),
        ("%G-W%V-%uT%H:%M:%S%.f", "ISO datetime (week date)"),
        ("%Y-%m-%d %H:%M:%S%.f", "Readable datetime"),
        ("%Y-%m-%d %H:%M", "Readable datetime"),
        ("%Y/%m/%d %H:%M:%S%.f", "Readable datetime"),
    ] {
        for suffix in ["%:z", "%z", " %:z", " %z", "%#z", " %#z"] {
            if let Ok(date) = DateTime::parse_from_str(input, &format!("{base}{suffix}")) {
                return Some(explicit(date, label));
            }
        }
        if let Some(utc_input) = input.strip_suffix('Z') {
            if let Ok(date) = NaiveDateTime::parse_from_str(utc_input, base) {
                return Some(explicit(date.and_utc().fixed_offset(), label));
            }
        } else if let Ok(date) = NaiveDateTime::parse_from_str(input, base) {
            return Some(local(date, label, zone));
        }
    }
    for format in ["%Y-%m-%d", "%Y/%m/%d", "%Y-%j", "%G-W%V-%u"] {
        if let Ok(date) = NaiveDate::parse_from_str(input, format) {
            let midnight = date.and_hms_opt(0, 0, 0).expect("midnight is valid");
            return Some(local(midnight, "Date only", zone));
        }
    }
    None
}

fn parse_timestamp(input: &str, unsigned: &str, unit: Option<Unit>) -> Result<Parsed, String> {
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || (unsigned.contains('.') && fraction.is_empty())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err("invalid numeric timestamp".into());
    }
    let unit = unit.unwrap_or_else(|| {
        if unsigned.contains('.') {
            return Unit::Seconds;
        }
        match whole.trim_start_matches('0').len() {
            0..=10 => Unit::Seconds,
            11..=13 => Unit::Milliseconds,
            14..=16 => Unit::Microseconds,
            _ => Unit::Nanoseconds,
        }
    });
    let scale = unit.scale();
    let precision = scale.ilog10() as usize;
    if fraction.len() > precision {
        return Err(format!(
            "{} supports at most {precision} fractional digits (nanosecond precision)",
            unit.label()
        ));
    }
    let overflow =
        || "timestamp is outside the supported date range (UTC years 0001–9999)".to_string();
    let whole: i128 = whole.parse().map_err(|_| overflow())?;
    let fractional_nanos = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i128>().map_err(|_| overflow())?
            * 10_i128.pow((precision - fraction.len()) as u32)
    };
    let nanos = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fractional_nanos))
        .ok_or_else(overflow)?;
    let nanos = if input.starts_with('-') {
        -nanos
    } else {
        nanos
    };
    let seconds = i64::try_from(nanos.div_euclid(NANOS_PER_SECOND)).map_err(|_| overflow())?;
    let subsecond = nanos.rem_euclid(NANOS_PER_SECOND) as u32;
    let datetime = DateTime::from_timestamp(seconds, subsecond).ok_or_else(overflow)?;
    checked(datetime, unit.label(), InputKind::Timestamp, Zone::Local)
}

fn explicit(date: DateTime<FixedOffset>, label: &'static str) -> Result<Parsed, String> {
    checked(
        date.to_utc(),
        label,
        InputKind::Datetime,
        Zone::Fixed(*date.offset()),
    )
}

fn local(date: NaiveDateTime, label: &'static str, zone: Zone) -> Result<Parsed, String> {
    checked(zone.resolve(date)?, label, InputKind::Datetime, zone)
}

fn checked(
    datetime: DateTime<Utc>,
    format: &'static str,
    kind: InputKind,
    timezone: Zone,
) -> Result<Parsed, String> {
    if !(1..=9999).contains(&datetime.year()) {
        return Err("date is outside the supported range (UTC years 0001–9999)".into());
    }
    if datetime.nanosecond() >= 1_000_000_000 {
        return Err("leap seconds cannot be represented unambiguously as Unix timestamps".into());
    }
    Ok(Parsed {
        datetime,
        format,
        kind,
        timezone,
    })
}
