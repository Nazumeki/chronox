use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use chronox::style::{self, ERROR, HEADING, MUTED};
use chronox::timezone::Zone;
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
    about = "Convert timestamps and dates with automatic format detection.\nDefault output: timestamp to local datetime; date to Unix seconds.\nUse now to show the current Unix seconds and local datetime.",
    before_help = format!("{HEADING}chronox{HEADING:#}  {MUTED}{}{MUTED:#}", env!("CARGO_PKG_VERSION")),
    help_template = "{before-help}{about}\n\n{usage-heading} {usage}\n\n{all-args}"
)]
struct Cli {
    #[arg(
        allow_negative_numbers = true,
        help = "Timestamp, date string, or now for the current time.\nQuote dates that contain spaces. Omit to read one value from stdin."
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
        help = "Choose the output format; overrides the default.\nUse s, ms, us, ns, readable, iso8601, http, or email.\nCannot be combined with --all.\n"
    )]
    to: Option<Format>,

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
        help = "Set the timezone for output and dates without a zone.\nUse an IANA name, UTC, local, or +/-HH:MM.\nDefault: input timezone, otherwise system timezone.\nHTTP dates always use GMT.\n"
    )]
    timezone: Option<Zone>,

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
    let parsed = parse::parse(&input, cli.unit, cli.timezone)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let mut stdout = anstream::AutoStream::auto(io::stdout());
    write!(stdout, "{}", output::render(&parsed, cli.to, cli.all))?;
    stdout.flush()
}
