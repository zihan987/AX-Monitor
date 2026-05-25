use std::process::ExitCode;
use std::time::Duration;

mod metrics;
mod model;
mod tui;

use metrics::MetricReader;

#[derive(Debug)]
struct Config {
    interval: Duration,
    once: bool,
    plain: bool,
}

fn main() -> ExitCode {
    let config = match parse_args(std::env::args().skip(1)) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            eprintln!("Usage: axmon [--once] [--plain] [--interval-ms N]");
            return ExitCode::from(2);
        }
    };

    let mut reader = MetricReader::default();
    if config.once || config.plain {
        tui::print_plain(&reader.snapshot());
        return ExitCode::SUCCESS;
    }

    match tui::run_dynamic(reader, config.interval) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("axmon: {err}");
            ExitCode::from(1)
        }
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Config, String> {
    let mut interval_ms = 1000_u64;
    let mut once = false;
    let mut plain = false;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--once" => once = true,
            "--plain" => plain = true,
            "--help" | "-h" => {
                return Err("AX Monitor".to_string());
            }
            "--interval-ms" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--interval-ms requires a value".to_string())?;
                interval_ms = value
                    .parse::<u64>()
                    .map_err(|_| "--interval-ms must be an integer".to_string())?
                    .max(100);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Config {
        interval: Duration::from_millis(interval_ms),
        once,
        plain,
    })
}

