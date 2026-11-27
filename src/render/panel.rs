use super::text::{Line, Tone, fit, human, middle_preserving, safe};
use crate::report::{Entry, EntryKind};
use unicode_width::UnicodeWidthStr;

pub(super) struct Panel<'a> {
    pub title: &'a str,
    pub entries: &'a [Entry],
    pub total: u64,
    pub partial: bool,
    pub top: usize,
}

impl Panel<'_> {
    pub(super) fn draw(&self, width: usize, rows: usize) -> Vec<Line> {
        let inside = width - 2;
        let content = width - 4;
        let shown = self.entries.len().min(self.top);
        let largest = self
            .entries
            .iter()
            .map(|entry| entry.size)
            .max()
            .unwrap_or(0);
        let caption = fit(
            &format!(" {} {shown} / {} ", self.title, self.entries.len()),
            inside,
        );
        let caption = if caption.ends_with(' ') {
            format!("{} ", caption.trim_end())
        } else {
            caption
        };
        let mut title = Line::new("╭", Tone::Muted);
        title.push(&caption, Tone::Title);
        title.push(
            format!("{}╮", "─".repeat(inside - caption.width())),
            Tone::Muted,
        );
        let mut lines = vec![title];

        let rank = if width >= 32 {
            shown.max(1).to_string().len().max(2)
        } else {
            0
        };
        let size = 10;
        let share = if width >= 42 { 7 } else { 0 };
        let name = content
            - size
            - 2
            - if rank > 0 { rank + 2 } else { 0 }
            - if share > 0 { share + 2 } else { 0 };
        let mut heading = Line::new("│ ", Tone::Muted);
        if rank > 0 {
            heading.push(format!("{}  ", fit("#", rank)), Tone::Muted);
        }
        heading.push(fit("NAME", name), Tone::Muted);
        heading.push(format!("  {:>size$}", "SIZE"), Tone::Muted);
        if share > 0 {
            heading.push(
                format!(
                    "  {:>share$}",
                    if self.partial { "% KNOWN" } else { "% TOTAL" }
                ),
                Tone::Muted,
            );
        }
        heading.push(" │", Tone::Muted);
        lines.push(heading);
        lines.push(rule(inside, '─'));

        for index in 0..rows {
            let mut row = Line::new("│ ", Tone::Muted);
            if let Some(entry) = self.entries.get(index).filter(|_| index < shown) {
                if rank > 0 {
                    row.push(format!("{:0rank$}  ", index + 1), Tone::Muted);
                }
                let suffix = match entry.kind {
                    EntryKind::Folder => "/",
                    EntryKind::Link => " [LINK]",
                    EntryKind::File => "",
                };
                let filename = safe(&entry.name);
                let keep_tail = suffix.width()
                    + if entry.kind == EntryKind::Folder {
                        0
                    } else {
                        extension(&filename).width()
                    };
                row.push(
                    middle_preserving(&format!("{filename}{suffix}"), name, keep_tail),
                    Tone::Text,
                );
                let heat = if self.top > 3 {
                    heat(entry.size, largest)
                } else {
                    None
                };
                row.push(
                    format!("  {:>size$}", human(entry.size)),
                    heat.unwrap_or(Tone::Size),
                );
                if share > 0 {
                    let percentage = if self.total == 0 {
                        "—".to_string()
                    } else {
                        format!("{:.1}%", entry.size as f64 / self.total as f64 * 100.0)
                    };
                    row.push(
                        format!("  {:>share$}", percentage),
                        heat.unwrap_or(Tone::Share),
                    );
                }
            } else {
                row.push(
                    fit(if index == 0 { "Empty" } else { "" }, content),
                    Tone::Muted,
                );
            }
            row.push(" │", Tone::Muted);
            lines.push(row);
        }
        lines.push(Line::new(format!("╰{}╯", "─".repeat(inside)), Tone::Muted));
        lines
    }
}

/// Normalize by the panel maximum, so changing --top does not recolor a row.
fn heat(size: u64, largest: u64) -> Option<Tone> {
    if largest == 0 {
        return None;
    }
    let weight = size as f64 / largest as f64;
    if weight <= 0.20 {
        return None;
    }
    let (start, end, fraction) = if weight <= 0.55 {
        ([165, 232, 125], [243, 204, 96], (weight - 0.20) / 0.35)
    } else {
        ([243, 204, 96], [255, 64, 64], (weight - 0.55) / 0.45)
    };
    Some(Tone::Heat(std::array::from_fn(|i| {
        (start[i] as f64 + (end[i] as f64 - start[i] as f64) * fraction).round() as u8
    })))
}

/// Retain short compound extensions such as .tar.zst as one recognizable suffix.
fn extension(filename: &str) -> &str {
    let Some((stem, last)) = filename.rsplit_once('.') else {
        return "";
    };
    if stem.is_empty() {
        return "";
    }
    if last.len() <= 4 {
        if let Some((base, previous)) = stem.rsplit_once('.') {
            if !base.is_empty() && previous.len() <= 4 {
                return &filename[base.len()..];
            }
        }
    }
    &filename[stem.len()..]
}

fn rule(inside: usize, character: char) -> Line {
    Line::new(
        format!("│{}│", character.to_string().repeat(inside)),
        Tone::Muted,
    )
}
