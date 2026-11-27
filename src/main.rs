use dutop::{
    cli,
    render::{render, safe},
    scan::scan,
};
use std::{
    ffi::OsStr,
    io::{self, IsTerminal, Write},
    num::NonZeroUsize,
    path::PathBuf,
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

struct Progress(bool);

impl Progress {
    fn start() -> Self {
        let active = io::stderr().is_terminal()
            && std::env::var_os("TERM").as_deref() != Some(OsStr::new("dumb"));
        if active {
            let _ = write!(io::stderr(), "Scanning disk usage… (Ctrl+C cancels)");
            let _ = io::stderr().flush();
        }
        Self(active)
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if self.0 {
            let _ = write!(io::stderr(), "\r\x1b[K");
        }
    }
}

fn run() -> io::Result<u8> {
    let raw: Vec<_> = std::env::args_os().collect();
    let args = match cli::command().try_get_matches_from(&raw) {
        Ok(args) => args,
        Err(error) => {
            use clap::error::ErrorKind::{DisplayHelp, DisplayVersion};
            if matches!(error.kind(), DisplayHelp | DisplayVersion) {
                print!("{error}");
                return Ok(0);
            }
            // Escape copies only for error presentation; successful paths keep their bytes.
            let escaped = raw.iter().map(|arg| safe(arg));
            match cli::command()
                .color(clap::ColorChoice::Never)
                .try_get_matches_from(escaped)
            {
                Err(display) => eprint!("{display}"),
                Ok(_) => eprintln!("Invalid arguments; run dutop --help."),
            }
            return Ok(2);
        }
    };
    let path = args
        .get_one::<PathBuf>("directory")
        .expect("default directory");
    let top = args
        .get_one::<NonZeroUsize>("top")
        .expect("default count")
        .get();
    let cancelled = Arc::new(AtomicBool::new(false));
    let output_phase = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register_conditional_shutdown(
        signal_hook::consts::SIGINT,
        130,
        Arc::clone(&output_phase),
    )?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&cancelled))?;
    let progress = Progress::start();
    let result = scan(path, &cancelled);
    drop(progress);
    let report = result.map_err(|error| {
        io::Error::new(error.kind(), format!("{}: {error}", safe(path.as_os_str())))
    })?;
    let stdout = io::stdout();
    let width = rustix::termios::tcgetwinsize(&stdout)
        .ok()
        .map(|size| usize::from(size.ws_col))
        .filter(|width| *width > 0)
        .or_else(|| {
            std::env::var("COLUMNS")
                .ok()?
                .parse::<u16>()
                .ok()
                .filter(|n| *n > 0)
                .map(usize::from)
        })
        .unwrap_or(100);
    let color = stdout.is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").as_deref() != Some(OsStr::new("dumb"));
    // Output has no external state to unwind. SIGINT must also stop a blocked pipe.
    output_phase.store(true, Ordering::Relaxed);
    let output = render(&report, width, top, color);
    if cancelled.load(Ordering::Relaxed) {
        return Err(io::ErrorKind::Interrupted.into());
    }
    writeln!(stdout.lock(), "{output}")?;
    Ok(u8::from(report.partial))
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) if error.kind() == io::ErrorKind::Interrupted => {
            eprintln!("Scan cancelled.");
            ExitCode::from(130)
        }
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dutop: {}", safe(OsStr::new(&error.to_string())));
            ExitCode::FAILURE
        }
    }
}
