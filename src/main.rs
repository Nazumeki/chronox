use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

use chronox::{output, parse};
use clap::{CommandFactory, Parser};
use output::Format;
use parse::Unit;

/// Recognize timestamps and dates, and convert them instantly.
#[derive(Parser)]
#[command(
    version,
    after_help = "EXAMPLES:\n  chronox 1704164645\n  chronox -a 1704164645123456789\n  chronox \"2024-01-02 03:04:05\"\n  chronox --unit ms -- -1\n  echo 1704164645 | chronox -a\n\nDefaults: datetime -> Unix timestamp (s); timestamp -> human-readable local datetime.\nDates without a timezone use the system timezone; explicit offsets or IANA names win. Numeric unit detection is a heuristic;\nuse --unit for ambiguous values. Quote dates containing spaces."
)]
struct Cli {
    /// Timestamp or date string (reads standard input if omitted)
    #[arg(allow_hyphen_values = true)]
    input: Option<String>,

    /// Render all supported output formats
    #[arg(short = 'a', long)]
    all: bool,

    /// Choose an output format, independent of the recognized input
    #[arg(short, long, value_enum, conflicts_with = "all")]
    to: Option<Format>,

    /// Override the detected numeric timestamp unit
    #[arg(short, long, value_enum)]
    unit: Option<Unit>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(cli);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = anstream::AutoStream::auto(io::stderr());
            let _ = writeln!(stderr, "\x1b[1;31merror:\x1b[0m {error}");
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
    let parsed = parse::parse(&input, cli.unit)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let mut stdout = anstream::AutoStream::auto(io::stdout());
    write!(
        stdout,
        "{}",
        output::render(
            &parsed,
            cli.to.unwrap_or_else(|| Format::default_for(&parsed)),
            cli.all
        )
    )?;
    stdout.flush()
}
