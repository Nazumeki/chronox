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
fn stdin_is_supported() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_chronox"))
        .args(["--to", "nanoseconds"])
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
            .contains("1704164645123456789")
    );
}

#[test]
fn invalid_inputs_and_conflicting_flags_fail_cleanly() {
    for args in [
        vec!["nonsense"],
        vec![],
        vec!["0", "-a", "--to", "http"],
        vec!["--unit", "invalid", "0"],
        vec!["0", "--to", "human"],
        vec!["0", "--to", "database"],
        vec!["0", "--to", "sql"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(!output.stderr.is_empty());
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn help_and_version_are_available() {
    for arg in ["--help", "--version"] {
        let output = run(&[arg]);
        assert!(output.status.success());
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("chronox")
        );
    }
}
