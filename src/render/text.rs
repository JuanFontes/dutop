use std::{ffi::OsStr, fmt::Write, os::unix::ffi::OsStrExt};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Escape filename bytes and controls rather than allowing terminal instructions.
pub fn safe(value: &OsStr) -> String {
    let mut output = String::new();
    for chunk in value.as_bytes().utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\n' => output.push_str("\\n"),
                '\r' => output.push_str("\\r"),
                '\t' => output.push_str("\\t"),
                c if c.is_control()
                    || matches!(
                        get_general_category(c),
                        GeneralCategory::Format
                            | GeneralCategory::LineSeparator
                            | GeneralCategory::ParagraphSeparator
                    ) =>
                {
                    let _ = write!(output, "\\u{{{:x}}}", c as u32);
                }
                c => output.push(c),
            }
        }
        for byte in chunk.invalid() {
            let _ = write!(output, "\\x{byte:02x}");
        }
    }
    output
}

pub(super) fn fit(value: &str, width: usize) -> String {
    if value.width() <= width {
        return format!("{value}{}", " ".repeat(width - value.width()));
    }
    if width == 0 {
        return String::new();
    }
    let (prefix, used) = take_cells(value, width - 1);
    format!("{prefix}…{}", " ".repeat(width - used - 1))
}

/// Keep both ends so extensions and the trailing link marker survive clipping.
pub(super) fn middle(value: &str, width: usize) -> String {
    middle_preserving(value, width, 0)
}

pub(super) fn middle_preserving(value: &str, width: usize, keep_tail: usize) -> String {
    if value.width() <= width || width < 2 {
        return fit(value, width);
    }
    let tail_budget = (width / 3).max(8).max(keep_tail).min(width - 1);
    let mut tail = "";
    for (index, _) in value.grapheme_indices(true).rev() {
        let candidate = &value[index..];
        if candidate.width() > tail_budget {
            break;
        }
        tail = candidate;
    }
    let tail_width = tail.width();
    let (head, head_width) = take_cells(value, width - tail_width - 1);
    format!(
        "{head}…{tail}{}",
        " ".repeat(width - head_width - tail_width - 1)
    )
}

fn take_cells(value: &str, width: usize) -> (String, usize) {
    let full_width = value.width();
    if full_width <= width {
        return (value.to_string(), full_width);
    }
    let mut end = 0;
    let mut used = 0;
    for (index, cluster) in value.grapheme_indices(true) {
        let next = index + cluster.len();
        let candidate_width = value[..next].width();
        if candidate_width > width {
            break;
        }
        end = next;
        used = candidate_width;
    }
    (value[..end].to_string(), used)
}

pub(super) fn human(bytes: u64) -> String {
    let mut value = bytes as f64;
    for unit in ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"] {
        if value < 1024.0 || unit == "EiB" {
            return if unit == "B" {
                format!("{bytes} B")
            } else {
                format!("{value:.1} {unit}")
            };
        }
        value /= 1024.0;
    }
    unreachable!()
}

#[derive(Clone, Copy)]
pub(super) enum Tone {
    Text,
    Muted,
    Title,
    Size,
    Total,
    Share,
    Heat([u8; 3]),
}

impl Tone {
    fn write_ansi(self, output: &mut String) {
        if let Self::Heat([r, g, b]) = self {
            let _ = write!(output, "\x1b[38;2;{r};{g};{b}m");
            return;
        }
        output.push_str(match self {
            Self::Text => "\x1b[38;2;227;231;232m",
            Self::Muted => "\x1b[38;2;128;149;159m",
            Self::Title => "\x1b[1;38;2;103;223;229m",
            Self::Size => "\x1b[38;2;165;232;125m",
            Self::Total => "\x1b[1;38;2;165;232;125m",
            Self::Share => "\x1b[38;2;162;222;221m",
            Self::Heat(_) => unreachable!(),
        });
    }
}

#[derive(Default)]
pub(super) struct Line(Vec<(String, Tone)>);

impl Line {
    pub(super) fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self(vec![(text.into(), tone)])
    }
    pub(super) fn push(&mut self, text: impl Into<String>, tone: Tone) {
        self.0.push((text.into(), tone));
    }
    pub(super) fn append(&mut self, other: Self) {
        self.0.extend(other.0);
    }
    pub(super) fn width(&self) -> usize {
        self.0.iter().map(|(text, _)| text.width()).sum()
    }

    /// Clip in terminal cells before emitting ANSI, never through an escape code.
    pub(super) fn finish(self, width: usize, color: bool) -> String {
        let clipped = self.width() > width;
        let mut remaining = width.saturating_sub(usize::from(clipped));
        let mut out = String::new();
        for (text, tone) in self.0 {
            let (visible, used) = take_cells(&text, remaining);
            if !visible.is_empty() {
                if color {
                    tone.write_ansi(&mut out);
                }
                out.push_str(&visible);
                if color {
                    out.push_str("\x1b[0m");
                }
            }
            remaining -= used;
            if used < text.width() {
                break;
            }
        }
        if clipped && width > 0 {
            if color {
                Tone::Muted.write_ansi(&mut out);
            }
            out.push('…');
            if color {
                out.push_str("\x1b[0m");
            }
        }
        out
    }
}
