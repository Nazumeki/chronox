# chronox

Recognize a timestamp or date string and render it in your chosen format. Built in
Rust with colored terminal output, automatic detection, and nanosecond precision.
Every recognized input can produce any output format through a shared UTC instant.

## Install

Install a current stable Rust toolchain, then run from this repository:

```sh
cargo install --path . --locked
```

Or use `cargo run -- <arguments>` while developing.

## Usage

```sh
chronox 1704164645123456789
chronox -a "2024-01-02 03:04:05.123456789"
chronox "Tue, 02 Jan 2024 11:04:05 +0800" --to milliseconds
chronox 1704164645 --to http
chronox --unit ns -- -1
echo 1704164645 | chronox -a
```

The default output adapts to the recognized input:

- Datetime input produces an integer Unix timestamp in **seconds**.
- Timestamp input produces a **readable datetime in your system timezone**:
  `YYYY-MM-DD HH:mm:ss +HH:mm`, with fractional seconds when present. Each value
  includes its UTC offset, and the timezone row identifies the detected timezone.

For example, on a system in Asia/Singapore, `chronox 1704164645` produces
`2024-01-02 11:04:05 +08:00`. Both `chronox "2024-01-02 11:04:05"` and
`chronox "2024-01-02T11:04:05+08:00"` produce `1704164645` on that system.

Use `-t, --to` to select an output, or `-a, --all` to render all eight formats.
These options are mutually exclusive. Quote input containing spaces; stdin accepts
one value, with surrounding whitespace ignored.

Example on a system in Asia/Singapore: `chronox -a 1704164645123456789`

```text
chronox  / time, translated

  Detected            Unix timestamp (ns)
  Timezone            Local (UTC+08:00)

  Unix timestamp (s)  1704164645
  Unix timestamp (ms) 1704164645123
  Unix timestamp (us) 1704164645123456
  Unix timestamp (ns) 1704164645123456789
  Readable datetime   2024-01-02 11:04:05.123456789 +08:00
  ISO datetime        2024-01-02T11:04:05.123456789+08:00
  HTTP date           Tue, 02 Jan 2024 03:04:05 GMT
  Email date          Tue, 2 Jan 2024 11:04:05 +0800

Integer timestamps round down; HTTP / email dates omit fractions.
```

Color adapts to the terminal and respects `NO_COLOR`. Redirected output is plain
text. Invalid input produces an error on stderr and exit code 2.

## Formats

| Input family | Examples |
| --- | --- |
| Unix timestamp (s) | `1704164645`, `1704164645.123456789` |
| Unix timestamp (ms) | `1704164645123` |
| Unix timestamp (us) | `1704164645123456` |
| Unix timestamp (ns) | `1704164645123456789` |
| ISO datetime | `2024-01-02T03:04:05Z`, `2024-01-02T11:04:05+08:00` |
| ISO basic, ordinal, week dates | `20240102T030405Z`, `2024-002T03:04:05Z`, `2024-W01-2T03:04:05Z` |
| Readable datetime | `2024-01-02 03:04:05.123456`, `2024/01/02 03:04:05`, `2024-01-02 11:04:05+08` |
| HTTP date | `Tue, 02 Jan 2024 03:04:05 GMT`, `Tuesday, 02-Jan-24 03:04:05 GMT`, `Tue Jan  2 03:04:05 2024` |
| Email date | `Tue, 02 Jan 2024 11:04:05 +0800` |
| Date only | `2024-01-02`, `2024/01/02`, `2024-002`, `2024-W01-2` |

ISO calendar and readable datetimes also accept hours and minutes without seconds.
Fractions support up to nine digits. Dates without a timezone use the system's local
timezone at the supplied date; date-only inputs use local midnight. Unix timestamps
always represent instants since the UTC epoch; timezone detection affects datetime
display, not the timestamp value.
Header inputs are the date values, without the `Date:` field name.

### Automatic timezone recognition

Explicit numeric offsets, `Z`, and standard HTTP/email timezone fields are recognized.
Datetime input also accepts a trailing `UTC`, `GMT`, or IANA timezone name:

```sh
chronox "2024-01-02 11:04:05 Asia/Singapore"
chronox "2024-07-02 12:00:00 America/New_York"
chronox "2024-01-02 03:04:05 UTC"
```

Readable, ISO, and email output preserve an explicit input timezone, or use the
system timezone when none is supplied. Each output includes an offset: `+08:00`
for readable and ISO output, and `+0800` for email. ISO output uses `Z` for UTC.
The timezone row shows the detected timezone and resolved offset.

HTTP dates always use GMT, as required by the HTTP format. Unix timestamps remain
independent of timezone. All rows in `-a` output represent the same instant, subject
to each format's precision.

Timezone rules are applied for the input date, including daylight saving. Ambiguous
or nonexistent local times during clock changes are rejected; provide an explicit
offset to identify the intended instant. If both an offset and IANA name are given,
they must agree. Regional abbreviations such as `CST` are not guessed outside the
standard email grammar; use an IANA name or numeric offset.

| Category | Format name | `--to` value (alias) | Example |
| --- | --- | --- | --- |
| Timestamp | Unix timestamp (s) | `seconds` (`s`) | `1704164645` |
| Timestamp | Unix timestamp (ms) | `milliseconds` (`ms`) | `1704164645123` |
| Timestamp | Unix timestamp (us) | `microseconds` (`us`) | `1704164645123456` |
| Timestamp | Unix timestamp (ns) | `nanoseconds` (`ns`) | `1704164645123456789` |
| Datetime | Readable datetime | `readable` | `2024-01-02 11:04:05.123456789 +08:00` |
| Datetime | ISO datetime | `iso8601` (`rfc3339`) | `2024-01-02T11:04:05.123456789+08:00` |
| Header date | HTTP date | `http` | `Tue, 02 Jan 2024 03:04:05 GMT` |
| Header date | Email date | `email` (`rfc2822`) | `Tue, 2 Jan 2024 11:04:05 +0800` |

Examples use the same instant; lower-precision formats omit fractional detail.
`us` denotes microseconds and matches the ASCII CLI alias.

### Numeric detection and precision

Integer detection uses the number of digits in the magnitude, ignoring the sign
and leading zeros: up to 10 means seconds, 11–13 milliseconds, 14–16 microseconds,
and 17 or more nanoseconds. Decimal input defaults to seconds.

Magnitude cannot identify units with certainty. Use `-u, --unit` with `seconds`,
`milliseconds`, `microseconds`, or `nanoseconds` (or `s`, `ms`, `us`, `ns`) for
small values near the epoch or dates outside the usual range. For example,
`chronox --unit ms 1000` means one second after the epoch. Pure digits always take
the timestamp path; use separators for a date-only input.

Integer output rounds down toward negative infinity when the target unit is
coarser than the input. HTTP and email formats have whole-second precision.
Readable, ISO, and nanosecond output preserve the complete instant. Calculations
use integers, including `i128` for nanoseconds, rather than floating point.

Supported UTC years are 0001–9999. Email output requires a year from 1900 to 9999
in the detected timezone.
Historical timezone offsets containing seconds cannot be represented by the
minute-resolution offsets used here. Readable, ISO, and email output report this
as unavailable; HTTP and Unix output still represent the instant accurately.
Leap seconds and fractions beyond nanosecond precision are rejected explicitly.
Ambiguous locale dates such as `01/02/2024` are rejected.

## Development and extension

```sh
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

- `src/parse.rs`: recognizes inputs and produces `Parsed`, containing a UTC instant,
  input kind, timezone, and detection metadata. Add a date recognizer to `RECOGNIZERS`; return `None` for
  a non-match or a validation result for a match. Register specific grammars before
  overlapping general ones. Numeric parsing separately handles the unit override.
- `src/output.rs`: defines output formats independently of input syntax. Add a
  `Format` variant, label, formatter, and precision rule. Clap's derived format
  registry automatically includes it in both `--to` and `-a`. `Format::default_for`
  selects the default from the input kind; explicit targets bypass that selection.
- `src/timezone.rs`: resolves system-local times, named timezones, and fixed offsets,
  detects ambiguous/nonexistent wall-clock times, and labels display timezones.
- `src/main.rs`: handles arguments, stdin, terminal styling, and exit codes.
- `src/lib.rs`: exposes parsing and formatting for reuse without launching the CLI.

New input formats do not need output-specific converters, and new output formats
do not need changes to existing recognizers. Tests exercise the full format matrix,
offset normalization, precision, pre-epoch values, range boundaries, and CLI behavior.
