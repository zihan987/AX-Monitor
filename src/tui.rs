use std::io::{self, Read, Write};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::metrics::MetricReader;
use crate::model::{Meter, Snapshot};

pub fn print_plain(snapshot: &Snapshot) {
    println!("AX Monitor");
    for meter in &snapshot.meters {
        println!(
            "{:<9} [{}] {:<8} {}",
            meter.title,
            bar(meter.percent, 28, false),
            meter.value,
            meter.detail
        );
    }
}

pub fn run_dynamic(mut reader: MetricReader, interval: Duration) -> io::Result<()> {
    let _terminal = TerminalGuard::enter()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        let mut buf = [0_u8; 1];
        while input.read_exact(&mut buf).is_ok() {
            if tx.send(buf[0]).is_err() {
                break;
            }
        }
    });

    loop {
        render(&reader.snapshot(), interval)?;
        let deadline = std::time::Instant::now() + interval;
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(b'q') | Ok(27) | Ok(3) => return Ok(()),
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }
}

fn render(snapshot: &Snapshot, interval: Duration) -> io::Result<()> {
    let mut out = io::stdout().lock();
    write!(out, "\x1b[H")?;
    writeln!(
        out,
        "AX Monitor                                 {}",
        timestamp_text()
    )?;
    writeln!(
        out,
        "Interval: {:>4} ms                         q/Esc quit",
        interval.as_millis()
    )?;
    writeln!(out)?;

    for meter in &snapshot.meters {
        render_meter(&mut out, meter)?;
    }

    writeln!(out)?;
    writeln!(out, "{}", " ".repeat(78))?;
    out.flush()
}

fn render_meter(out: &mut impl Write, meter: &Meter) -> io::Result<()> {
    writeln!(
        out,
        "{:<9} {}  {:>8}  {}{}",
        meter.title,
        bar(meter.percent, 30, true),
        meter.value,
        meter.detail,
        clear_line()
    )
}

fn bar(percent: Option<f64>, width: usize, color: bool) -> String {
    let Some(percent) = percent else {
        return ".".repeat(width);
    };
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let color_code = if percent >= 85.0 {
        "\x1b[31m"
    } else if percent >= 65.0 {
        "\x1b[33m"
    } else {
        "\x1b[32m"
    };

    if color {
        format!(
            "{color_code}{}\x1b[90m{}\x1b[0m",
            "█".repeat(filled),
            "░".repeat(empty)
        )
    } else {
        format!("{}{}", "#".repeat(filled), "-".repeat(empty))
    }
}

fn clear_line() -> &'static str {
    "\x1b[K"
}

fn timestamp_text() -> String {
    if let Ok(output) = Command::new("date").arg("+%F %T").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }
    }

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("epoch {secs}")
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        let _ = Command::new("stty")
            .args(["-icanon", "-echo", "min", "1", "time", "0"])
            .status();
        let mut out = io::stdout().lock();
        write!(out, "\x1b[?1049h\x1b[2J\x1b[H\x1b[?25l")?;
        out.flush()?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = Command::new("stty").args(["sane"]).status();
        let mut out = io::stdout().lock();
        let _ = write!(out, "\x1b[?25h\x1b[?1049l");
        let _ = out.flush();
    }
}
