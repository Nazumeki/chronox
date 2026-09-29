# chronox

English | [简体中文](README.zh-CN.md)

A command-line tool written in Rust for converting Unix timestamps and date
strings. It detects input formats and supports nanosecond precision.

## Installation

With a current stable Rust toolchain installed, run from the repository root:

```sh
cargo install --path . --locked
```

## Usage

```sh
chronox 1704164645
chronox "2024-01-02T03:04:05Z" --to milliseconds
chronox -a 1704164645123456789
echo 1704164645 | chronox -z UTC
```

By default, timestamps produce a readable datetime in the system timezone, and
date strings produce an integer Unix timestamp in seconds. Quote inputs that
contain spaces. If no argument is supplied, chronox reads one value from stdin
and ignores surrounding whitespace.

| Option | Description |
| --- | --- |
| `-t, --to FORMAT` | Select an output format from the table below. |
| `-a, --all` | Show all eight output formats. Cannot be combined with `--to`. |
| `-u, --unit UNIT` | Set the unit for numeric input: `s`, `ms`, `us`, or `ns`, or the corresponding full name. |
| `-z, --timezone ZONE` | Set the output timezone and the timezone used to interpret dates without one. |
| `-h, --help` | Show help. |
| `-v, --version` | Show the version. |

Output includes the detected input format and display timezone. Color follows
terminal settings and respects `NO_COLOR`; redirected output is plain text.
Invalid input produces an error on stderr and exit code 2.

## Supported formats

### Input

| Format | Examples |
| --- | --- |
| Unix timestamp | `1704164645`, `1704164645.123456789` |
| ISO datetime | `2024-01-02T03:04:05Z`, `2024-01-02T11:04:05+08:00` |
| ISO basic datetime | `20240102T030405Z` |
| ISO ordinal or week datetime | `2024-002T03:04:05Z`, `2024-W01-2T03:04:05Z` |
| Readable datetime | `2024-01-02 03:04:05.123456`, `2024/01/02 03:04:05` |
| HTTP date | `Tue, 02 Jan 2024 03:04:05 GMT` |
| Email date | `Tue, 02 Jan 2024 11:04:05 +0800` |
| Date only | `2024-01-02`, `2024/01/02`, `2024-002`, `2024-W01-2` |

Timestamps accept seconds, milliseconds, microseconds, and nanoseconds; see
[unit detection](#units-and-precision). ISO calendar and hyphen-separated readable
datetimes also accept hours and minutes without seconds. Date-only inputs use
midnight in the applicable timezone.

HTTP input also accepts the older RFC 850 and asctime forms. HTTP and email inputs
must contain only the date value, without the `Date:` field name.

### Output

| Format | `--to` value | Alias |
| --- | --- | --- |
| Unix seconds | `seconds` | `s` |
| Unix milliseconds | `milliseconds` | `ms` |
| Unix microseconds | `microseconds` | `us` |
| Unix nanoseconds | `nanoseconds` | `ns` |
| Readable datetime | `readable` | — |
| ISO datetime | `iso8601` | `rfc3339` |
| HTTP date | `http` | — |
| Email date | `email` | `rfc2822` |

Readable output uses `YYYY-MM-DD HH:mm:ss +HH:mm`, with fractional seconds when
present. ISO output uses RFC 3339. Readable, ISO, and email output include a UTC
offset; ISO uses `Z` for UTC. HTTP output always uses GMT.

## Timezones

Input can specify a numeric offset, `Z`, a standard HTTP/email timezone field,
or a trailing `UTC`, `GMT`, or IANA name such as `Asia/Singapore`.

Use `-z, --timezone` to select an IANA name, `UTC` (also `GMT` or `Z`), `local`,
or a fixed offset such as `+08:00`, `-05:00`, or `+0545`.

```sh
chronox "2024-01-02 11:04:05 Asia/Singapore" -z UTC --to iso8601
chronox 1704164645 -z=-05:00
```

Timezone selection follows these rules:

- An explicit input timezone determines the instant. `--timezone` changes how
  that instant is displayed.
- Dates without a timezone use `--timezone` if supplied, otherwise the system
  timezone.
- Without `--timezone`, output uses the explicit input timezone if present,
  otherwise the system timezone.
- Unix timestamps are independent of timezone. All output formats represent the
  same instant, within each format's precision.

Named timezones apply daylight-saving rules for the input date; fixed offsets do
not change. Ambiguous or nonexistent local times are rejected. Supply an explicit
offset to identify the intended instant. If an input includes both an offset and
an IANA name, they must agree. Regional abbreviations such as `CST` are accepted
only where defined by the email date grammar.

## Units and precision

Integer timestamp units are inferred from the digit count, excluding the sign
and leading zeros:

| Digits | Unit |
| --- | --- |
| 1–10 | Seconds |
| 11–13 | Milliseconds |
| 14–16 | Microseconds |
| 17 or more | Nanoseconds |

Zero and decimal input default to seconds. Use `--unit` when digit count does not
identify the intended unit: `chronox --unit ms 1000` means one second after the
Unix epoch. For negative input, `--` can separate options from the value:
`chronox --unit ns -- -1`. Pure digits are always treated as timestamps; use
separators for date-only input.

Calculations use integers and retain nanosecond precision. Integer timestamp
output rounds down toward negative infinity when converting to a coarser unit.
HTTP and email output omit fractional seconds. Readable, ISO, and nanosecond
output preserve full precision when the format is available.

UTC years 0001–9999 are supported. Email output requires a year from 1900 to 9999
in the display timezone. Historical offsets containing seconds cannot be
represented in readable, ISO, or email output; those formats are marked
unavailable, while HTTP and Unix output remain available. Leap seconds,
precision beyond nanoseconds, and ambiguous date forms such as `01/02/2024`
are rejected.

## Development

Run locally with `cargo run -- <arguments>`. To check changes:

```sh
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI runs these checks and documentation tests on Linux, Windows, and macOS
(Apple Silicon and Intel). Dependabot checks Cargo dependencies and GitHub Actions
weekly, grouping minor/patch Rust updates and action updates into separate PRs.

### Coverage

CI generates coverage with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).
Each successful coverage job includes a summary and a `coverage` artifact with
LCOV and HTML reports, retained for 14 days. Download and extract the artifact,
then open `html/index.html` to browse coverage. No external service or token is
required. To generate an HTML report locally:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --locked --all-targets --html
```

Open `target/llvm-cov/html/index.html`. Coverage is reported without enforcing a
minimum percentage.

### Releases

Update the version in `Cargo.toml`, run `cargo check` to refresh `Cargo.lock`,
and commit both files. Push a matching tag, for example:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow verifies the tag against the crate version and runs CI before
building binaries. It publishes a GitHub Release with generated notes, SHA-256
checksums (`SHA256SUMS`), and archives containing the binary, license, and READMEs:

| Platform | Target | Archive |
| --- | --- | --- |
| Linux x86-64 (glibc, built on Ubuntu 24.04) | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Windows x86-64 | `x86_64-pc-windows-msvc` | `.zip` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.tar.gz` |
| macOS Intel | `x86_64-apple-darwin` | `.tar.gz` |

Prerelease versions such as `0.2.0-rc.1` produce GitHub prereleases. Publishing uses
the built-in `GITHUB_TOKEN`; no additional secrets are needed. This workflow
publishes binaries to GitHub Releases, not the crate to crates.io.

### Source layout

Parsing and formatting share a UTC instant, so input and output formats can be
extended independently.

| File | Responsibility |
| --- | --- |
| [src/parse.rs](src/parse.rs) | Input recognition, unit detection, and validation. |
| [src/output.rs](src/output.rs) | Output formats, default selection, and rendering. |
| [src/timezone.rs](src/timezone.rs) | Timezone resolution and local-time validation. |
| [src/main.rs](src/main.rs) | Arguments, stdin, and exit codes. |
| [src/style.rs](src/style.rs) | Terminal colors and help styling. |
| [src/lib.rs](src/lib.rs) | Public parsing and formatting API. |
