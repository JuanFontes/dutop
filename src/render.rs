//! Terminal-only presentation. No filesystem access or subprocesses.

mod panel;
mod text;

use crate::report::Report;
use panel::Panel;
pub use text::safe;
use text::{Line, Tone, human, middle};
use unicode_width::UnicodeWidthStr;

/// Render at most N actual entries per panel, with bounded empty-row padding.
pub fn render(report: &Report, width: usize, top: usize, color: bool) -> String {
    let width = width.max(1);
    let top = top.clamp(1, crate::MAX_TOP);
    let side_by_side = width >= 90;
    let box_width = if side_by_side {
        (width - 3) / 2
    } else {
        width.max(20)
    };
    let folder_rows = top.min(10.max(report.directories.len()));
    let file_rows = top.min(10.max(report.files.len()));
    let rows = folder_rows.max(file_rows);
    let folders = Panel {
        title: "FOLDERS",
        entries: &report.directories,
        total: report.total,
        partial: report.partial,
        top,
    }
    .draw(box_width, if side_by_side { rows } else { folder_rows });
    let files = Panel {
        title: "FILES",
        entries: &report.files,
        total: report.total,
        partial: report.partial,
        top,
    }
    .draw(box_width, if side_by_side { rows } else { file_rows });

    let mut heading = Line::new("dutop ", Tone::Title);
    let path = safe(report.path.as_os_str());
    let mode = "ALLOCATED SPACE";
    if width >= 60 {
        let space = width - 6 - mode.width() - 3;
        let path = middle(&path, space.min(path.width()));
        heading.push(path, Tone::Text);
        heading.push(
            " ".repeat(width - heading.width() - mode.width()),
            Tone::Muted,
        );
        heading.push(mode, Tone::Muted);
    } else {
        heading.push(path, Tone::Text);
    }
    let mut lines = vec![heading, Line::default()];
    if side_by_side {
        lines.extend(folders.into_iter().zip(files).map(|(mut left, right)| {
            left.push("   ", Tone::Muted);
            left.append(right);
            left
        }));
    } else {
        lines.extend(folders);
        lines.push(Line::default());
        lines.extend(files);
    }
    lines.push(Line::default());
    lines.push(Line::new("─".repeat(width), Tone::Muted));
    let mut total = Line::new(
        if report.partial {
            "PARTIAL TOTAL "
        } else {
            "TOTAL "
        },
        Tone::Title,
    );
    total.push(human(report.total), Tone::Total);
    let counts = [
        format!("{} folders", report.directories.len()),
        format!("{} files/links", report.files.len()),
    ];
    for count in counts {
        if total.width() + 4 + count.width() > width {
            lines.push(total);
            total = Line::new(count, Tone::Text);
        } else {
            total.push(format!("    {count}"), Tone::Text);
        }
    }
    lines.push(total);
    lines.push(Line::new("─".repeat(width), Tone::Muted));
    let denominator = if report.partial {
        "% uses partial total"
    } else {
        "% uses full directory total"
    };
    let mut hints = vec!["Hidden included", denominator, "symlink targets excluded"];
    if width < 60 {
        hints.insert(0, "Allocated disk space");
    }
    lines.extend(wrap_hints(&hints, width));
    if report.partial {
        lines.push(Line::new(
            format!("{} path/read errors; total is incomplete.", report.skipped),
            Tone::Title,
        ));
    }
    lines
        .into_iter()
        .map(|line| line.finish(width, color))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_hints(hints: &[&str], width: usize) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for hint in hints {
        if !current.is_empty() && current.width() + 3 + hint.width() > width {
            lines.push(Line::new(current, Tone::Muted));
            current = String::new();
        }
        if !current.is_empty() {
            current.push_str(" · ");
        }
        current.push_str(hint);
    }
    lines.push(Line::new(current, Tone::Muted));
    lines
}
