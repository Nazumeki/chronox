use chrono::{DateTime, Local, TimeZone, Utc};
use chronox::{
    output::Format,
    parse::{Unit, parse},
};
use clap::ValueEnum;

fn utc(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input).unwrap().to_utc()
}

#[test]
fn detects_all_timestamp_scales_without_floating_point_loss() {
    for (input, expected, label) in [
        ("1704164645", "2024-01-02T03:04:05Z", "Unix timestamp (s)"),
        (
            "1704164645123",
            "2024-01-02T03:04:05.123Z",
            "Unix timestamp (ms)",
        ),
        (
            "1704164645123456",
            "2024-01-02T03:04:05.123456Z",
            "Unix timestamp (us)",
        ),
        (
            "1704164645123456789",
            "2024-01-02T03:04:05.123456789Z",
            "Unix timestamp (ns)",
        ),
        (
            "1704164645.123456789",
            "2024-01-02T03:04:05.123456789Z",
            "Unix timestamp (s)",
        ),
        (
            "+0001704164645",
            "2024-01-02T03:04:05Z",
            "Unix timestamp (s)",
        ),
    ] {
        let parsed = parse(input, None).unwrap();
        assert_eq!(parsed.datetime, utc(expected), "{input}");
        assert_eq!(parsed.format, label);
    }
}

#[test]
fn recognizes_datetime_families_and_normalizes_offsets() {
    for input in [
        "2024-01-02T03:04:05Z",
        "2024-01-02T11:04:05+08:00",
        "2024-01-01T22:04:05-05:00",
        "20240102T030405Z",
        "2024-002T03:04:05Z",
        "2024-W01-2T03:04:05Z",
        "2024-01-02 11:04:05+0800",
        "2024-01-02 11:04:05 +08:00",
        "2024-01-02 11:04:05+08",
        "Tue, 02 Jan 2024 03:04:05 GMT",
        "Tue, 02 Jan 2024 11:04:05 +0800",
        "Tuesday, 02-Jan-24 03:04:05 GMT",
        "Tue Jan  2 03:04:05 2024",
    ] {
        assert_eq!(
            parse(input, None).unwrap().datetime,
            utc("2024-01-02T03:04:05Z"),
            "{input}"
        );
    }
}

#[test]
fn missing_timezones_use_system_local_time() {
    assert_eq!(
        parse("2024-01-02T03:04Z", None).unwrap().datetime,
        utc("2024-01-02T03:04:00Z")
    );
    assert_eq!(
        parse("2024-01-02 03:04", None).unwrap().datetime,
        Local
            .with_ymd_and_hms(2024, 1, 2, 3, 4, 0)
            .unwrap()
            .to_utc()
    );
    for input in ["2024-01-02", "2024/01/02", "2024-002", "2024-W01-2"] {
        let parsed = parse(input, None).unwrap();
        assert!(matches!(parsed.timezone, chronox::timezone::Zone::Local));
        assert_eq!(
            parsed.datetime,
            Local
                .with_ymd_and_hms(2024, 1, 2, 0, 0, 0)
                .unwrap()
                .to_utc()
        );
    }
    for input in [
        "2024-01-02 03:04:05",
        "2024-01-02T03:04:05",
        "2024/01/02 03:04:05",
    ] {
        assert_eq!(
            parse(input, None).unwrap().datetime,
            Local
                .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
                .unwrap()
                .to_utc()
        );
    }
}

#[test]
fn negative_fractions_and_explicit_units_are_exact() {
    for (input, unit, nanos) in [
        ("-0.000000001", Unit::Seconds, -1),
        ("-1", Unit::Milliseconds, -1_000_000),
        ("-1", Unit::Microseconds, -1_000),
        ("-1", Unit::Nanoseconds, -1),
        ("1.000001", Unit::Milliseconds, 1_000_001),
        ("1.001", Unit::Microseconds, 1_001),
        ("0", Unit::Seconds, 0),
    ] {
        assert_eq!(parse(input, Some(unit)).unwrap().nanos(), nanos);
    }
    let parsed = parse("-1", Some(Unit::Nanoseconds)).unwrap();
    assert_eq!(
        parse(&Format::Iso8601.value(&parsed), None)
            .unwrap()
            .datetime,
        utc("1969-12-31T23:59:59.999999999Z")
    );
    for target in [Format::Seconds, Format::Milliseconds, Format::Microseconds] {
        assert_eq!(target.value(&parsed), "-1");
    }
}

#[test]
fn every_input_can_render_every_output() {
    let cases = [
        (Format::Seconds, "1704164645"),
        (Format::Milliseconds, "1704164645000"),
        (Format::Microseconds, "1704164645000000"),
        (Format::Nanoseconds, "1704164645000000000"),
        (Format::Iso8601, "2024-01-02T03:04:05Z"),
        (Format::Readable, "2024-01-02 03:04:05 +00:00"),
        (Format::Http, "Tue, 02 Jan 2024 03:04:05 GMT"),
        (Format::Email, "Tue, 2 Jan 2024 03:04:05 +0000"),
    ];
    for (_, input) in cases {
        let parsed = parse(input, None).unwrap();
        for (target, expected) in cases {
            let value = target.value(&parsed);
            if matches!(target, Format::Readable | Format::Iso8601 | Format::Email) {
                let reparsed = parse(&value, None).unwrap();
                assert_eq!(reparsed.datetime, parsed.datetime, "{input} -> {value}");
                assert_eq!(
                    reparsed.timezone.at(reparsed.datetime).offset(),
                    parsed.timezone.at(parsed.datetime).offset()
                );
            } else {
                assert_eq!(value, expected, "{input} -> {target:?}");
            }
        }
    }
}

#[test]
fn default_output_depends_on_input_kind() {
    for input in [
        "0",
        "1704164645123",
        "1704164645123456",
        "1704164645123456789",
        "-0.1",
    ] {
        assert_eq!(
            Format::default_for(&parse(input, None).unwrap()),
            Format::Readable
        );
    }
    for input in [
        "2024-01-02",
        "2024-01-02T03:04:05Z",
        "Tue, 02 Jan 2024 03:04:05 GMT",
    ] {
        assert_eq!(
            Format::default_for(&parse(input, None).unwrap()),
            Format::Seconds
        );
    }
}

#[test]
fn named_timezones_and_offsets_identify_the_same_instant() {
    for input in [
        "2024-01-02 11:04:05 Asia/Singapore",
        "2024-01-02T11:04:05+08:00 Asia/Singapore",
        "2024-01-02 08:49:05 Asia/Kathmandu",
        "2024-01-02 03:04:05 UTC",
        "2024-01-02 03:04:05 GMT",
        "2024-01-01 22:04:05 America/New_York",
    ] {
        assert_eq!(
            parse(input, None).unwrap().datetime,
            utc("2024-01-02T03:04:05Z"),
            "{input}"
        );
    }
    let parsed = parse("2024-01-02T11:04:05.123456789+08:00", None).unwrap();
    assert_eq!(parsed.timezone.label(parsed.datetime), "UTC+08:00");
    assert_eq!(
        Format::Readable.value(&parsed),
        "2024-01-02 11:04:05.123456789 +08:00"
    );
    let summer = parse("2024-07-02 12:00:00 America/New_York", None).unwrap();
    assert_eq!(
        Format::Readable.value(&summer),
        "2024-07-02 12:00:00 -04:00"
    );
    assert!(summer.timezone.label(summer.datetime).contains("-04:00"));
    let winter = parse("2024-01-02 12:00:00 America/New_York", None).unwrap();
    assert_eq!(
        Format::Readable.value(&winter),
        "2024-01-02 12:00:00 -05:00"
    );
    assert!(winter.timezone.label(winter.datetime).contains("-05:00"));
}

#[test]
fn datetime_outputs_preserve_offsets_and_http_stays_gmt() {
    for (input, readable, iso, email) in [
        (
            "2024-01-02 08:34:05 Asia/Kolkata",
            "2024-01-02 08:34:05 +05:30",
            "2024-01-02T08:34:05+05:30",
            "Tue, 2 Jan 2024 08:34:05 +0530",
        ),
        (
            "2024-01-02 11:04:05 Asia/Singapore",
            "2024-01-02 11:04:05 +08:00",
            "2024-01-02T11:04:05+08:00",
            "Tue, 2 Jan 2024 11:04:05 +0800",
        ),
        (
            "2024-01-02 08:49:05 Asia/Kathmandu",
            "2024-01-02 08:49:05 +05:45",
            "2024-01-02T08:49:05+05:45",
            "Tue, 2 Jan 2024 08:49:05 +0545",
        ),
        (
            "2024-01-01T22:04:05-05:00",
            "2024-01-01 22:04:05 -05:00",
            "2024-01-01T22:04:05-05:00",
            "Mon, 1 Jan 2024 22:04:05 -0500",
        ),
        (
            "2024-01-02T03:04:05Z",
            "2024-01-02 03:04:05 +00:00",
            "2024-01-02T03:04:05Z",
            "Tue, 2 Jan 2024 03:04:05 +0000",
        ),
    ] {
        let parsed = parse(input, None).unwrap();
        assert_eq!(Format::Readable.value(&parsed), readable);
        assert_eq!(Format::Iso8601.value(&parsed), iso);
        assert_eq!(Format::Email.value(&parsed), email);
        assert_eq!(Format::Http.value(&parsed), "Tue, 02 Jan 2024 03:04:05 GMT");
        assert_eq!(Format::Seconds.value(&parsed), "1704164645");
    }
}

#[test]
fn copied_datetimes_preserve_precision_and_dst_fold_identity() {
    for input in [
        "2024-01-02T11:04:05.123456789+08:00",
        "2024-11-03T01:30:00-04:00 America/New_York",
        "2024-11-03T01:30:00-05:00 America/New_York",
    ] {
        let parsed = parse(input, None).unwrap();
        for format in [Format::Readable, Format::Iso8601] {
            let output = format.value(&parsed);
            assert_eq!(
                parse(&output, None).unwrap().nanos(),
                parsed.nanos(),
                "{output}"
            );
        }
    }
}

#[test]
fn historical_offsets_are_not_silently_rounded() {
    let parsed = parse("1900-01-02 12:00:00 Asia/Singapore", None).unwrap();
    for format in [Format::Readable, Format::Iso8601, Format::Email] {
        assert!(format.value(&parsed).contains("offset includes seconds"));
    }
    assert!(Format::Http.value(&parsed).ends_with("GMT"));
    assert_eq!(
        parse(&Format::Nanoseconds.value(&parsed), Some(Unit::Nanoseconds))
            .unwrap()
            .datetime,
        parsed.datetime
    );
}

#[test]
fn email_year_limits_follow_the_display_timezone() {
    let before = parse("1899-12-31T23:30:00-01:00", None).unwrap();
    assert!(Format::Email.value(&before).starts_with("unavailable"));
    let after = parse("1900-01-01T00:30:00+01:00", None).unwrap();
    assert_eq!(
        Format::Email.value(&after),
        "Mon, 1 Jan 1900 00:30:00 +0100"
    );
}

#[test]
fn timezone_transitions_and_conflicts_are_not_guessed() {
    for (input, error) in [
        ("2024-11-03 01:30:00 America/New_York", "ambiguous"),
        ("2024-03-10 02:30:00 America/New_York", "does not exist"),
        ("2024-01-02T11:04:05+00:00 Asia/Singapore", "conflicts"),
        ("2024-01-02 11:04:05 Asia/Unknown", "unknown timezone"),
    ] {
        assert!(parse(input, None).unwrap_err().contains(error), "{input}");
    }
    let first = parse("2024-11-03T01:30:00-04:00 America/New_York", None).unwrap();
    let second = parse("2024-11-03T01:30:00-05:00 America/New_York", None).unwrap();
    assert_eq!((second.datetime - first.datetime).num_seconds(), 3600);
    assert!(parse("2024-01-02 03:04:05 CST", None).is_err());
}

#[test]
fn dates_outside_i64_nanoseconds_still_render() {
    for input in ["0001-01-01T00:00:00Z", "9999-12-31T23:59:59.999999999Z"] {
        let parsed = parse(input, None).unwrap();
        let nanos = Format::Nanoseconds.value(&parsed);
        assert_eq!(
            parse(&nanos, Some(Unit::Nanoseconds)).unwrap().datetime,
            parsed.datetime
        );
        for format in Format::value_variants() {
            assert!(!format.value(&parsed).is_empty());
        }
    }
}

#[test]
fn rejects_invalid_ambiguous_and_unrepresentable_values() {
    for input in [
        "",
        "   ",
        "tomorrow",
        "01/02/2024",
        "2024-02-30",
        "2023-02-29",
        "2024-01-02T25:00:00Z",
        "2024-01-02T03:04:05+25:00",
        "2024-01-02T03:04:05.1234567891Z",
        "1.1234567890",
        "1.2.3",
        ".5",
        "1.",
        "2016-12-31T23:59:60Z",
        "9999999999999999999999999999999999999999999999",
        "0000-01-01T00:00:00Z",
        "2024-01-02\n2024-01-03",
    ] {
        assert!(parse(input, None).is_err(), "accepted {input:?}");
    }
    assert!(parse("2024-01-02", Some(Unit::Seconds)).is_err());
    assert!(parse("0.1", Some(Unit::Nanoseconds)).is_err());
}
