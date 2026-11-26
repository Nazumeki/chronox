use std::fmt::Write;

use chrono::{DateTime, Datelike, Months, SecondsFormat, Utc};
use clap::ValueEnum;

use crate::parse::{InputKind, Parsed};
use crate::style::{HEADING, LABEL, MUTED, VALUE, WARNING};
use crate::timezone::Zone;

/// Every output format accepts the same normalized instant, regardless of input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Unix timestamp in seconds
    #[value(name = "seconds", alias = "s")]
    Seconds,
    /// Unix timestamp in milliseconds
    #[value(name = "milliseconds", alias = "ms")]
    Milliseconds,
    /// Unix timestamp in microseconds
    #[value(name = "microseconds", alias = "us")]
    Microseconds,
    /// Unix timestamp in nanoseconds
    #[value(name = "nanoseconds", alias = "ns")]
    Nanoseconds,
    /// Readable datetime in the detected timezone
    #[value(name = "readable")]
    Readable,
    /// ISO datetime in the detected timezone
    #[value(name = "iso8601", alias = "rfc3339")]
    Iso8601,
    /// HTTP date in GMT
    #[value(name = "http")]
    Http,
    /// Email date in the detected timezone
    #[value(name = "email", alias = "rfc2822")]
    Email,
    /// Human-readable offset relative to the current time
    #[value(name = "relative", alias = "rel")]
    Relative,
}

impl Format {
    pub fn defaults_for(parsed: &Parsed) -> &'static [Self] {
        match parsed.kind {
            InputKind::Timestamp => &[Self::Readable],
            InputKind::Datetime => &[Self::Seconds],
            InputKind::Now | InputKind::Relative => &[Self::Seconds, Self::Readable],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Seconds => "Unix timestamp (s)",
            Self::Milliseconds => "Unix timestamp (ms)",
            Self::Microseconds => "Unix timestamp (us)",
            Self::Nanoseconds => "Unix timestamp (ns)",
            Self::Readable => "Readable datetime",
            Self::Iso8601 => "ISO datetime",
            Self::Http => "HTTP date",
            Self::Email => "Email date",
            Self::Relative => "Relative time",
        }
    }

    pub fn value(self, parsed: &Parsed) -> String {
        // These formats carry minute-resolution offsets. Historical timezone
        // offsets can include seconds; do not silently round them to another instant.
        if matches!(self, Self::Readable | Self::Iso8601 | Self::Email)
            && parsed
                .timezone
                .at(parsed.datetime)
                .offset()
                .local_minus_utc()
                % 60
                != 0
        {
            return "unavailable (historical timezone offset includes seconds)".into();
        }
        match self {
            Self::Seconds => parsed.nanos().div_euclid(1_000_000_000).to_string(),
            Self::Milliseconds => parsed.nanos().div_euclid(1_000_000).to_string(),
            Self::Microseconds => parsed.nanos().div_euclid(1_000).to_string(),
            Self::Nanoseconds => parsed.nanos().to_string(),
            Self::Readable => parsed
                .timezone
                .at(parsed.datetime)
                .format("%Y-%m-%d %H:%M:%S%.f %:z")
                .to_string(),
            Self::Iso8601 => parsed
                .timezone
                .at(parsed.datetime)
                .to_rfc3339_opts(SecondsFormat::AutoSi, true),
            Self::Http => parsed
                .datetime
                .format("%a, %d %b %Y %H:%M:%S GMT")
                .to_string(),
            Self::Email => {
                let date = parsed.timezone.at(parsed.datetime);
                if (1900..=9999).contains(&date.year()) {
                    date.to_rfc2822()
                } else {
                    "unavailable (email dates require a local year from 1900 to 9999)".into()
                }
            }
            Self::Relative => format_relative(parsed.datetime, Utc::now()),
        }
    }

    fn loses_precision(self, parsed: &Parsed) -> bool {
        let resolution = match self {
            Self::Seconds | Self::Http | Self::Email | Self::Relative => 1_000_000_000,
            Self::Milliseconds => 1_000_000,
            Self::Microseconds => 1_000,
            _ => 1,
        };
        parsed.nanos().rem_euclid(resolution) != 0
    }
}

pub fn render(parsed: &Parsed, target: Option<Format>, all: bool, zones: &[Zone]) -> String {
    let width = Format::value_variants()
        .iter()
        .map(|format| format.label().len())
        .max()
        .unwrap_or(0)
        .max("Timezone".len());
    let mut output = format!("{HEADING}chronox{HEADING:#}  {MUTED}time, translated{MUTED:#}\n\n");
    row(&mut output, "Detected", parsed.format, width);

    let formats = selected_formats(parsed, target.as_ref(), all, zones);

    if zones.len() >= 2 {
        output.push('\n');
        for (index, zone) in zones.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            writeln!(
                output,
                "{HEADING}{}{HEADING:#}",
                zone.label(parsed.datetime)
            )
            .expect("writing to a String cannot fail");
            let mut view = parsed.clone();
            view.timezone = *zone;
            for format in formats {
                write_row(&mut output, *format, &view, width);
            }
        }
    } else {
        row(
            &mut output,
            "Timezone",
            &parsed.timezone.label(parsed.datetime),
            width,
        );
        output.push('\n');
        let heading = if all { "All formats" } else { "Result" };
        writeln!(output, "{HEADING}{heading}{HEADING:#}").expect("writing to a String cannot fail");
        for format in formats {
            write_row(&mut output, *format, parsed, width);
        }
    }

    if formats.iter().any(|format| format.loses_precision(parsed)) {
        writeln!(output, "\n{WARNING}Note:{WARNING:#} Integer timestamps round down; HTTP, email, and relative formats omit fractions.")
            .expect("writing to a String cannot fail");
    }
    output
}

pub fn render_raw(parsed: &Parsed, target: Option<Format>, all: bool, zones: &[Zone]) -> String {
    let formats = selected_formats(parsed, target.as_ref(), all, zones);
    let mut output = String::new();
    if zones.len() >= 2 {
        for zone in zones {
            let mut view = parsed.clone();
            view.timezone = *zone;
            for format in formats {
                writeln!(output, "{}", format.value(&view))
                    .expect("writing to a String cannot fail");
            }
        }
    } else {
        for format in formats {
            writeln!(output, "{}", format.value(parsed)).expect("writing to a String cannot fail");
        }
    }
    output
}

pub fn format_relative(instant: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let instant_nanos = nanos_of(instant);
    let now_nanos = nanos_of(now);
    let delta = instant_nanos - now_nanos;
    if delta.unsigned_abs() < 1_000_000_000 {
        return "now".into();
    }
    let (earlier, later, future) = if delta > 0 {
        (now, instant, true)
    } else {
        (instant, now, false)
    };

    let mut total_months =
        (later.year() - earlier.year()) * 12 + (later.month() as i32 - earlier.month() as i32);
    if total_months > 0
        && earlier
            .checked_add_months(Months::new(total_months as u32))
            .is_none_or(|advanced| advanced > later)
    {
        total_months -= 1;
    }
    let anchor = earlier
        .checked_add_months(Months::new(total_months as u32))
        .unwrap_or(earlier);
    let rem_secs = ((nanos_of(later) - nanos_of(anchor)) / 1_000_000_000) as u64;

    let units = [
        ((total_months / 12) as u64, "year", "years"),
        ((total_months % 12) as u64, "month", "months"),
        (rem_secs / 86_400, "day", "days"),
        ((rem_secs % 86_400) / 3_600, "hour", "hours"),
        ((rem_secs % 3_600) / 60, "minute", "minutes"),
        (rem_secs % 60, "second", "seconds"),
    ];

    let primary_index = units
        .iter()
        .position(|(count, _, _)| *count > 0)
        .expect("delta is at least 1 second");
    let mut text = plural(units[primary_index]);
    if let Some(&next) = units.get(primary_index + 1)
        && next.0 > 0
    {
        let _ = write!(text, ", {}", plural(next));
    }
    if future {
        format!("in {text}")
    } else {
        format!("{text} ago")
    }
}

fn selected_formats<'a>(
    parsed: &Parsed,
    target: Option<&'a Format>,
    all: bool,
    zones: &[Zone],
) -> &'a [Format] {
    // CLI choices and -a share a registry so new formats appear in both.
    if all {
        Format::value_variants()
    } else if let Some(target) = target {
        std::slice::from_ref(target)
    } else if zones.len() >= 2 && parsed.kind == InputKind::Datetime {
        &[Format::Readable]
    } else {
        Format::defaults_for(parsed)
    }
}

fn nanos_of(datetime: DateTime<Utc>) -> i128 {
    i128::from(datetime.timestamp()) * 1_000_000_000 + i128::from(datetime.timestamp_subsec_nanos())
}

fn plural((count, singular, plural): (u64, &str, &str)) -> String {
    let noun = if count == 1 { singular } else { plural };
    format!("{count} {noun}")
}

fn write_row(output: &mut String, format: Format, parsed: &Parsed, width: usize) {
    let label = format!("{:<width$}", format.label());
    let value = format.value(parsed);
    writeln!(output, "  {LABEL}{label}{LABEL:#}  {VALUE}{value}{VALUE:#}")
        .expect("writing to a String cannot fail");
}

fn row(output: &mut String, label: &str, value: &str, width: usize) {
    let label = format!("{label:<width$}");
    writeln!(output, "  {MUTED}{label}{MUTED:#}  {value}")
        .expect("writing to a String cannot fail");
}
