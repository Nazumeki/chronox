use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_chronox"))
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn output_value<'a>(stdout: &'a str, label: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|line| line.trim_start().strip_prefix(label))
        .unwrap_or_else(|| panic!("missing {label}: {stdout}"))
        .trim()
}

#[test]
fn now_defaults_to_current_seconds_and_datetime_in_selected_timezone() {
    for zone in [None, Some("UTC"), Some("Asia/Singapore"), Some("-05:00")] {
        let mut args = vec!["now"];
        if let Some(zone) = zone {
            args.extend(["-z", zone]);
        }
        let before = chrono::Utc::now();
        let output = run(&args);
        let after = chrono::Utc::now();
        assert!(output.status.success(), "{:?}", output.stderr);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let seconds: i64 = output_value(&stdout, "Unix timestamp (s)").parse().unwrap();
        let datetime = chrono::DateTime::parse_from_str(
            output_value(&stdout, "Readable datetime"),
            "%Y-%m-%d %H:%M:%S%.f %:z",
        )
        .unwrap();
        assert!(datetime.to_utc() >= before && datetime.to_utc() <= after);
        assert_eq!(seconds, datetime.timestamp());
        let expected_zone: chronox::timezone::Zone = zone.unwrap_or("local").parse().unwrap();
        assert_eq!(
            datetime.offset(),
            expected_zone.at(datetime.to_utc()).offset()
        );
        assert!(stdout.contains("Current time"));
        assert!(!stdout.contains('\x1b'));
        assert!(!stdout.contains("Unix timestamp (ms)"));
        assert!(!stdout.contains("ISO datetime"));
    }
}

#[test]
fn now_supports_selected_and_all_output_formats() {
    let before = chrono::Utc::now();
    let output = run(&["now", "--to", "iso8601", "-z", "UTC"]);
    let after = chrono::Utc::now();
    assert!(output.status.success(), "{:?}", output.stderr);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let datetime = chrono::DateTime::parse_from_rfc3339(output_value(&stdout, "ISO datetime"))
        .unwrap()
        .to_utc();
    assert!(datetime >= before && datetime <= after);
    assert!(!stdout.contains("Unix timestamp (s)"));
    assert!(!stdout.contains("Readable datetime"));

    let before = chrono::Utc::now();
    let output = run(&["--all", "now", "-z", "UTC"]);
    let after = chrono::Utc::now();
    assert!(output.status.success(), "{:?}", output.stderr);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed = chronox::parse::parse(output_value(&stdout, "ISO datetime"), None, None).unwrap();
    assert!(parsed.datetime >= before && parsed.datetime <= after);
    for format in <chronox::output::Format as clap::ValueEnum>::value_variants() {
        assert_eq!(output_value(&stdout, format.label()), format.value(&parsed));
    }
}

#[test]
fn datetime_input_defaults_to_seconds() {
    for input in [
        "2024-01-02 03:04:05 UTC",
        "2024-01-02T11:04:05+08:00",
        "2024-01-02 11:04:05 Asia/Singapore",
        "Tue, 02 Jan 2024 03:04:05 GMT",
    ] {
        let output = run(&[input]);
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("1704164645"));
        assert!(stdout.contains("Unix timestamp (s)"));
        let values = stdout.split("\n\n").nth(2).unwrap();
        assert!(!values.contains("Readable datetime"));
        assert!(!stdout.contains('\x1b'));
    }
}

#[test]
fn timestamp_input_defaults_to_readable_local_datetime() {
    for input in [
        "1704164645",
        "1704164645000",
        "1704164645000000",
        "1704164645000000000",
    ] {
        let output = run(&[input]);
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let expected = chrono::DateTime::from_timestamp(1704164645, 0)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S%.f %:z")
            .to_string();
        assert!(stdout.contains(&expected));
        assert!(stdout.contains("Readable datetime"));
        assert!(stdout.contains("Local (UTC"));
        assert!(!stdout.contains("ISO datetime"));
    }
}

#[test]
fn explicit_target_overrides_automatic_selection() {
    let output = run(&["2024-01-02T11:04:05+08:00", "--to", "readable"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("2024-01-02 11:04:05 +08:00")
    );
    let output = run(&["1704164645", "--to", "iso8601"]);
    assert!(output.status.success());
    let expected = chrono::DateTime::from_timestamp(1704164645, 0)
        .unwrap()
        .with_timezone(&chrono::Local)
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains(&expected)
    );
}

#[test]
fn selected_output_works_for_numeric_input() {
    let output = run(&["1704164645", "--to", "ms"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("1704164645000")
    );
}

#[test]
fn all_formats_and_negative_arguments_work() {
    let output = run(&["-a", "--unit", "ns", "-1"]);
    assert!(output.status.success(), "{:?}", output.stderr);
    let stdout = String::from_utf8(output.stdout).unwrap();
    for label in [
        "Unix timestamp (s)",
        "Unix timestamp (ms)",
        "Unix timestamp (us)",
        "Unix timestamp (ns)",
        "Readable datetime",
        "ISO datetime",
        "HTTP date",
        "Email date",
    ] {
        assert!(stdout.contains(label), "missing {label}");
    }
    let expected = chrono::DateTime::from_timestamp(-1, 999_999_999)
        .unwrap()
        .with_timezone(&chrono::Local)
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    assert!(stdout.contains(&expected));
}

#[test]
fn timezone_long_and_short_options_select_output_zone() {
    for args in [
        vec!["1704164645", "--timezone", "America/New_York"],
        vec!["1704164645", "-z", "America/New_York"],
        vec!["-z=America/New_York", "1704164645"],
        vec!["1704164645", "-z", "-05:00"],
        vec!["1704164645", "-z=-0500"],
        vec!["--timezone=-05:00", "1704164645"],
    ] {
        let output = run(&args);
        assert!(output.status.success(), "{args:?}: {:?}", output.stderr);
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("2024-01-01 22:04:05 -05:00")
        );
    }
    for zone in ["UTC", "utc", "GMT", "Z"] {
        let output = run(&["1704164645", "-z", zone]);
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("2024-01-02 03:04:05 +00:00")
        );
    }
    let output = run(&["1704164645", "-z", "local"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Local (UTC")
    );
}

#[test]
fn target_timezone_does_not_reinterpret_explicit_input_zones() {
    for input in [
        "2024-01-02T11:04:05+08:00",
        "2024-01-02 11:04:05 Asia/Singapore",
        "Tue, 02 Jan 2024 11:04:05 +0800",
        "Tue, 02 Jan 2024 03:04:05 GMT",
    ] {
        let output = run(&[input, "-z", "America/New_York", "-a"]);
        assert!(output.status.success(), "{input}: {:?}", output.stderr);
        let stdout = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "America/New_York (UTC-05:00)",
            "1704164645",
            "2024-01-01 22:04:05 -05:00",
            "2024-01-01T22:04:05-05:00",
            "Mon, 1 Jan 2024 22:04:05 -0500",
            "Tue, 02 Jan 2024 03:04:05 GMT",
        ] {
            assert!(stdout.contains(expected), "missing {expected}");
        }
    }
}

#[test]
fn target_timezone_interprets_naive_dates_and_applies_dst_rules() {
    for (input, zone, expected) in [
        ("2024-01-02 11:04:05", "Asia/Singapore", "1704164645"),
        ("2024-01-02 08:49:05", "+05:45", "1704164645"),
        ("2024-01-02", "UTC", "1704153600"),
    ] {
        let output = run(&[input, "-z", zone]);
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
    for (input, expected) in [
        ("2024-01-02T17:00:00Z", "2024-01-02 12:00:00 -05:00"),
        ("2024-07-02T16:00:00Z", "2024-07-02 12:00:00 -04:00"),
    ] {
        let output = run(&[input, "-z", "America/New_York", "--to", "readable"]);
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
    for input in ["2024-11-03 01:30:00", "2024-03-10 02:30:00"] {
        assert_eq!(
            run(&[input, "-z", "America/New_York"]).status.code(),
            Some(2)
        );
    }
    let output = run(&["2024-11-03T01:30:00-04:00", "-z", "UTC", "--to", "iso8601"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("2024-11-03T05:30:00Z")
    );
}

#[test]
fn invalid_timezone_arguments_fail_without_accepting_partial_offsets() {
    for zone in [
        "Asia/Unknown",
        "CST",
        "+24:00",
        "-25:00",
        "+08:60",
        "+08:00junk",
        "+08:00:00",
        "0800",
        "+8:00",
        "",
    ] {
        let output = run(&["1704164645", "-z", zone]);
        assert_eq!(output.status.code(), Some(2), "accepted {zone:?}");
        assert!(output.stdout.is_empty());
    }
    let output = run(&["1704164645", "-z"]);
    assert_eq!(output.status.code(), Some(2));
    let output = run(&["--", "-z"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unrecognized or invalid date")
    );
}

#[test]
fn stdin_is_supported() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_chronox"))
        .args(["--to", "readable", "-z", "+05:45"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"2024-01-02T03:04:05.123456789Z\r\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("2024-01-02 08:49:05.123456789 +05:45")
    );
}

#[test]
fn invalid_inputs_and_conflicting_flags_fail_cleanly() {
    for args in [
        vec!["nonsense"],
        vec![],
        vec!["0", "-a", "--to", "http"],
        vec!["now", "-a", "--to", "http"],
        vec!["now", "--unit", "s"],
        vec!["now", "extra"],
        vec!["--unit", "invalid", "0"],
        vec!["0", "--to", "human"],
        vec!["0", "--to", "database"],
        vec!["0", "--to", "sql"],
        vec!["-V"],
        vec!["1704164645", "-tz", "UTC"],
        vec!["1704164645", "-tz=UTC"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(!output.stderr.is_empty());
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn help_and_version_are_available() {
    for arg in ["--help", "--version", "-v"] {
        let output = run(&[arg]);
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("chronox")
        );
    }
}
