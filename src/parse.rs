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
    Now,
    Relative,
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

#[derive(Clone, Debug)]
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
    if let Some((radix, digits)) = radix_prefix(unsigned) {
        return parse_radix(input, digits, radix, unit);
    }
    if !unsigned.is_empty()
        && unsigned
            .bytes()
            .any(|byte| byte.is_ascii_digit() || byte == b'.')
        && unsigned
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.' || byte == b'_')
    {
        return parse_timestamp(input, unsigned, unit);
    }
    if unit.is_some() {
        return Err("--unit applies only to numeric Unix timestamps".into());
    }
    if input.eq_ignore_ascii_case("now") {
        return Ok(Parsed {
            datetime: Utc::now(),
            format: "Current time",
            kind: InputKind::Now,
            timezone: default_zone,
        });
    }
    if let Some(result) = relative(input, default_zone) {
        return result;
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
    let normalized = input.to_ascii_uppercase();
    let input = normalized.as_str();
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
    let (raw_whole, raw_fraction, has_dot) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, fraction, true),
        None => (unsigned, "", false),
    };
    let Some(whole_digits) = strip_underscores(raw_whole, u8::is_ascii_digit) else {
        return Err("invalid numeric timestamp".into());
    };
    let fraction_digits = if has_dot {
        let Some(digits) = strip_underscores(raw_fraction, u8::is_ascii_digit) else {
            return Err("invalid numeric timestamp".into());
        };
        digits
    } else {
        String::new()
    };
    let unit = unit.unwrap_or_else(|| {
        if has_dot {
            return Unit::Seconds;
        }
        match whole_digits.trim_start_matches('0').len() {
            0..=10 => Unit::Seconds,
            11..=13 => Unit::Milliseconds,
            14..=16 => Unit::Microseconds,
            _ => Unit::Nanoseconds,
        }
    });
    let scale = unit.scale();
    let precision = scale.ilog10() as usize;
    if fraction_digits.len() > precision {
        return Err(format!(
            "{} supports at most {precision} fractional digits (nanosecond precision)",
            unit.label()
        ));
    }
    let overflow_msg = "timestamp is outside the supported date range (UTC years 0001–9999)";
    let overflow = || overflow_msg.to_string();
    let whole: i128 = whole_digits.parse().map_err(|_| overflow())?;
    let fractional_nanos = if fraction_digits.is_empty() {
        0
    } else {
        fraction_digits.parse::<i128>().map_err(|_| overflow())?
            * 10_i128.pow((precision - fraction_digits.len()) as u32)
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
    let datetime = from_nanos(nanos, overflow_msg)?;
    checked(datetime, unit.label(), InputKind::Timestamp, Zone::Local)
}

#[derive(Clone, Copy)]
enum Radix {
    Hex,
    Octal,
    Binary,
}

impl Radix {
    fn base(self) -> u32 {
        match self {
            Self::Hex => 16,
            Self::Octal => 8,
            Self::Binary => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Hex => "hexadecimal",
            Self::Octal => "octal",
            Self::Binary => "binary",
        }
    }

    fn is_digit(self, byte: u8) -> bool {
        match self {
            Self::Hex => byte.is_ascii_hexdigit(),
            Self::Octal => matches!(byte, b'0'..=b'7'),
            Self::Binary => matches!(byte, b'0' | b'1'),
        }
    }

    fn label(self, unit: Unit) -> &'static str {
        match (self, unit) {
            (Self::Hex, Unit::Seconds) => "Hex timestamp (s)",
            (Self::Hex, Unit::Milliseconds) => "Hex timestamp (ms)",
            (Self::Hex, Unit::Microseconds) => "Hex timestamp (us)",
            (Self::Hex, Unit::Nanoseconds) => "Hex timestamp (ns)",
            (Self::Octal, Unit::Seconds) => "Octal timestamp (s)",
            (Self::Octal, Unit::Milliseconds) => "Octal timestamp (ms)",
            (Self::Octal, Unit::Microseconds) => "Octal timestamp (us)",
            (Self::Octal, Unit::Nanoseconds) => "Octal timestamp (ns)",
            (Self::Binary, Unit::Seconds) => "Binary timestamp (s)",
            (Self::Binary, Unit::Milliseconds) => "Binary timestamp (ms)",
            (Self::Binary, Unit::Microseconds) => "Binary timestamp (us)",
            (Self::Binary, Unit::Nanoseconds) => "Binary timestamp (ns)",
        }
    }
}

/// Split a `0x`/`0X`, `0o`/`0O`, or `0b`/`0B` prefix off an unsigned input.
fn radix_prefix(unsigned: &str) -> Option<(Radix, &str)> {
    if let Some(digits) = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
    {
        return Some((Radix::Hex, digits));
    }
    if let Some(digits) = unsigned
        .strip_prefix("0o")
        .or_else(|| unsigned.strip_prefix("0O"))
    {
        return Some((Radix::Octal, digits));
    }
    if let Some(digits) = unsigned
        .strip_prefix("0b")
        .or_else(|| unsigned.strip_prefix("0B"))
    {
        return Some((Radix::Binary, digits));
    }
    None
}

fn parse_radix(
    input: &str,
    digits: &str,
    radix: Radix,
    unit: Option<Unit>,
) -> Result<Parsed, String> {
    let digits = digits.strip_prefix('_').unwrap_or(digits);
    let clean = strip_underscores(digits, |byte| radix.is_digit(*byte))
        .ok_or_else(|| format!("invalid {} timestamp", radix.name()))?;
    let overflow_msg = "timestamp is outside the supported date range (UTC years 0001–9999)";
    let magnitude =
        i128::from_str_radix(&clean, radix.base()).map_err(|_| overflow_msg.to_string())?;
    let value = if input.starts_with('-') {
        -magnitude
    } else {
        magnitude
    };
    let unit = unit.unwrap_or(Unit::Seconds);
    let nanos = value
        .checked_mul(unit.scale())
        .ok_or_else(|| overflow_msg.to_string())?;
    let datetime = from_nanos(nanos, overflow_msg)?;
    checked(
        datetime,
        radix.label(unit),
        InputKind::Timestamp,
        Zone::Local,
    )
}

fn relative(input: &str, zone: Zone) -> Recognition {
    let rest = if input
        .as_bytes()
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"now"))
    {
        let after_now = input[3..].trim_start();
        if !after_now.starts_with(['+', '-']) {
            return None;
        }
        after_now
    } else if input.starts_with(['+', '-']) {
        let after_sign = input[1..].trim_start();
        if !after_sign
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_digit)
            || after_sign.contains([':', '-', '/'])
        {
            return None;
        }
        input
    } else {
        return None;
    };
    Some(parse_relative(rest, zone))
}

fn parse_relative(rest: &str, zone: Zone) -> Result<Parsed, String> {
    let invalid = || {
        "invalid relative time offset; use w, d, h, m, s, ms, us, or ns (for example now+2h or -30m)"
            .to_string()
    };
    let overflow_msg = "date is outside the supported range (UTC years 0001–9999)";
    let overflow = || overflow_msg.to_string();

    let negative = rest.starts_with('-');
    let mut cursor = rest[1..].trim_start();
    if cursor.is_empty() {
        return Err(invalid());
    }

    let mut total_nanos: i128 = 0;
    while !cursor.is_empty() {
        let num_len = cursor
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || *byte == b'_')
            .count();
        if num_len == 0 {
            return Err(invalid());
        }
        let digits =
            strip_underscores(&cursor[..num_len], u8::is_ascii_digit).ok_or_else(invalid)?;
        let amount: i128 = digits.parse().map_err(|_| overflow())?;
        cursor = &cursor[num_len..];

        let unit_len = cursor.bytes().take_while(u8::is_ascii_alphabetic).count();
        if unit_len == 0 {
            return Err(invalid());
        }
        let unit_scale: i128 = match cursor[..unit_len].to_ascii_lowercase().as_str() {
            "ns" => 1,
            "us" => 1_000,
            "ms" => 1_000_000,
            "s" => NANOS_PER_SECOND,
            "m" => 60 * NANOS_PER_SECOND,
            "h" => 3_600 * NANOS_PER_SECOND,
            "d" => 86_400 * NANOS_PER_SECOND,
            "w" => 604_800 * NANOS_PER_SECOND,
            _ => return Err(invalid()),
        };
        let part_nanos = amount.checked_mul(unit_scale).ok_or_else(overflow)?;
        total_nanos = total_nanos.checked_add(part_nanos).ok_or_else(overflow)?;
        cursor = cursor[unit_len..].trim_start();
    }

    let now = Utc::now();
    let now_nanos =
        i128::from(now.timestamp()) * NANOS_PER_SECOND + i128::from(now.timestamp_subsec_nanos());
    let target_nanos = if negative {
        now_nanos.checked_sub(total_nanos)
    } else {
        now_nanos.checked_add(total_nanos)
    }
    .ok_or_else(overflow)?;

    let datetime = from_nanos(target_nanos, overflow_msg)?;
    checked(datetime, "Relative time", InputKind::Relative, zone)
}

fn strip_underscores(input: &str, is_digit: impl Fn(&u8) -> bool) -> Option<String> {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut clean = String::with_capacity(bytes.len());
    for (index, byte) in bytes.iter().enumerate() {
        if is_digit(byte) {
            clean.push(*byte as char);
        } else if *byte == b'_'
            && index > 0
            && index + 1 < bytes.len()
            && is_digit(&bytes[index - 1])
            && is_digit(&bytes[index + 1])
        {
            continue;
        } else {
            return None;
        }
    }
    Some(clean)
}

fn from_nanos(nanos: i128, error: &'static str) -> Result<DateTime<Utc>, String> {
    let seconds =
        i64::try_from(nanos.div_euclid(NANOS_PER_SECOND)).map_err(|_| error.to_string())?;
    let subsecond = nanos.rem_euclid(NANOS_PER_SECOND) as u32;
    DateTime::from_timestamp(seconds, subsecond).ok_or_else(|| error.to_string())
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
