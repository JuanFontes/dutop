use clap::{Arg, Command, value_parser};
use std::{num::NonZeroUsize, path::PathBuf};

pub fn command() -> Command {
    Command::new("dutop")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Largest immediate folders and files, with complete allocated disk usage")
        .arg(
            Arg::new("directory")
                .value_name("DIRECTORY")
                .default_value(".")
                .value_parser(value_parser!(PathBuf))
                .help("Directory to scan; defaults to the current directory"),
        )
        .arg(
            Arg::new("top")
                .long("top")
                .short('n')
                .value_name("N")
                .default_value("10")
                .value_parser(parse_top)
                .help("Number of entries per panel (1–99)"),
        )
        .after_help(
            "Examples:\n  dutop\n  dutop --top 4 ~/Downloads\n  dutop --top 40\n\
Sizes measure allocated disk space, not file length. Hidden entries are included.\n\
Folder sizes include descendants; only immediate entries are listed. The total\n\
includes every entry, not just the top N. Child symlink targets are excluded.\n\
Percentages use the complete directory total; partial scans use the known total.\n\
Hard links count once in the total, so row sizes may not add up to it. Shared\n\
storage and snapshots mean totals do not predict space freed by deletion.\n\
Mounted filesystems are included. Ctrl+C cancels. Partial scans exit with status 1.\n\
With --top above 3, size and percentage colors scale from green through yellow\n\
to red relative to the largest entry in each panel. Small entries keep their\n\
normal colors. Displayed percentages still use the complete directory total.\n\
Colors respect NO_COLOR and TERM=dumb, and are disabled when output is piped.",
        )
}

fn parse_top(value: &str) -> Result<NonZeroUsize, String> {
    value
        .parse::<NonZeroUsize>()
        .ok()
        .filter(|n| n.get() <= crate::MAX_TOP)
        .ok_or_else(|| format!("must be a whole number between 1 and {}", crate::MAX_TOP))
}
