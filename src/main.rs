use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use chronox::style::{self, ERROR, HEADING, MUTED};
use chronox::timezone::{self, Zone};
use chronox::{output, parse};
use clap::{CommandFactory, Parser};
use output::Format;
use parse::Unit;

#[derive(Parser)]
#[command(
    version,
    styles = style::help(),
    disable_help_flag = true,
    disable_version_flag = true,
    about = "Convert timestamps and dates with automatic format detection.\nDefault output: timestamp to local datetime; date to Unix seconds.\nUse now or relative offsets (now+2h, -30m) for current or offset time.",
    before_help = format!("{HEADING}chronox{HEADING:#}  {MUTED}{}{MUTED:#}", env!("CARGO_PKG_VERSION")),
    help_template = "{before-help}{about}\n\n{usage-heading} {usage}\n\n{all-args}"
)]
struct Cli {
    #[arg(
        allow_negative_numbers = true,
        help = "Timestamp, date string, now, or relative offset (e.g. now+2h, +15m).\nQuote inputs that contain spaces. Omit to read one value from stdin."
    )]
    input: Option<String>,

    #[arg(short = 'a', long, help = "Show all timestamp and date formats.\n")]
    all: bool,

    #[arg(
        short,
        long,
        value_enum,
        value_name = "FORMAT",
        hide_possible_values = true,
        conflicts_with = "all",
        help = "Choose the output format; overrides the default.\nUse s, ms, us, ns, readable, iso8601, http, email, or relative.\nCannot be combined with --all.\n"
    )]
    to: Option<Format>,

    #[arg(
        short = 'r',
        long,
        help = "Print converted value(s) only, one per line.\n"
    )]
    raw: bool,

    #[arg(
        short,
        long,
        value_enum,
        hide_possible_values = true,
        help = "Override the detected unit for numeric input.\ns = seconds, ms = milliseconds,\nus = microseconds, ns = nanoseconds.\n"
    )]
    unit: Option<Unit>,

    #[arg(
        short = 'z',
        long,
        value_name = "ZONE",
        allow_hyphen_values = true,
        help = "Set the timezone for output and dates without a zone.\nRepeat for multiple output zones; the first interprets dates without a zone.\nUse an IANA name, UTC, local, or +/-HH:MM.\nDefault: input timezone, otherwise system timezone.\nHTTP dates always use GMT.\n"
    )]
    timezone: Vec<Zone>,

    #[arg(
        long,
        conflicts_with_all = ["input", "all", "to", "raw", "unit", "timezone"],
        help = "List all available IANA timezone names and exit."
    )]
    list_timezones: bool,

    #[arg(short = 'h', long, action = clap::ArgAction::Help, help = "Print help\n")]
    help: Option<bool>,

    #[arg(short = 'v', long, action = clap::ArgAction::Version, help = "Print version")]
    version: Option<bool>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(cli);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = anstream::AutoStream::auto(io::stderr());
            let _ = writeln!(stderr, "{ERROR}error:{ERROR:#} {error}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> io::Result<()> {
    if cli.list_timezones {
        let mut stdout = anstream::AutoStream::auto(io::stdout());
        for name in timezone::names() {
            writeln!(stdout, "{name}")?;
        }
        return stdout.flush();
    }
    let input = match cli.input {
        Some(input) => input,
        None if io::stdin().is_terminal() => {
            Cli::command().print_help()?;
            return Ok(());
        }
        None => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            input
        }
    };
    let parsed = parse::parse(&input, cli.unit, cli.timezone.first().copied())
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let rendered = if cli.raw {
        output::render_raw(&parsed, cli.to, cli.all, &cli.timezone)
    } else {
        output::render(&parsed, cli.to, cli.all, &cli.timezone)
    };
    let mut stdout = anstream::AutoStream::auto(io::stdout());
    write!(stdout, "{rendered}")?;
    stdout.flush()
}
