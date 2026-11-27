use dutop::{
    render::{render, safe},
    report::{Entry, EntryKind, Report},
};
use std::{
    ffi::{OsStr, OsString},
    os::unix::ffi::OsStringExt,
};
use unicode_width::UnicodeWidthStr;

fn report() -> Report {
    Report {
        path: "/example".into(),
        total: 1_048_576,
        partial: false,
        skipped: 0,
        directories: (0..45)
            .map(|i| Entry {
                name: format!("folder-{i:02}").into(),
                size: 65536 - i,
                kind: EntryKind::Folder,
            })
            .collect(),
        files: (0..45)
            .map(|i| Entry {
                name: format!("file-{i:02}").into(),
                size: 32768 - i,
                kind: EntryKind::File,
            })
            .collect(),
    }
}

#[test]
fn top_controls_each_panel_without_changing_complete_total() {
    let r = report();
    for n in [4, 10, 40] {
        let output = render(&r, 120, n, false);
        assert!(output.contains(&format!("FOLDERS {n} / 45")));
        assert!(output.contains(&format!("folder-{:02}/", n - 1)));
        assert!(output.contains(&format!("file-{:02}", n - 1)));
        assert!(!output.contains(&format!("folder-{n:02}/")));
        assert!(!output.contains(&format!("file-{n:02}")));
        assert!(output.contains("TOTAL 1.0 MiB"));
        assert!(output.contains("45 folders"));
    }
}

#[test]
fn panels_stack_on_small_terminals_and_never_overflow() {
    let mut r = report();
    r.files[0].name = "漢字-é-e\u{301}-long-file".into();
    for width in [1, 10, 20, 48, 89, 90, 120] {
        let output = render(&r, width, 4, false);
        assert!(!output.is_empty());
        assert!(
            output.lines().all(|line| line.width() <= width),
            "width {width}: {output}"
        );
        if width >= 90 {
            assert!(
                output
                    .lines()
                    .any(|line| line.contains("FOLDERS") && line.contains("FILES"))
            );
        } else if width >= 20 {
            assert!(
                !output
                    .lines()
                    .any(|line| line.contains("FOLDERS") && line.contains("FILES"))
            );
        }
    }
}

#[test]
fn filenames_cannot_inject_terminal_controls_or_bidi_and_invalid_bytes_are_visible() {
    let raw = OsString::from_vec(b"bad\xff\n\t\x1b[31m".to_vec());
    let escaped = safe(&raw);
    assert!(escaped.contains("\\xff"));
    assert!(escaped.contains("\\n"));
    assert!(!escaped.contains(['\n', '\t', '\x1b']));
    assert!(!safe(OsStr::new("hidden\u{202e}txt")).contains('\u{202e}'));
    let mut r = report();
    r.path = raw.clone().into();
    r.files[0].name = raw;
    assert!(!render(&r, 120, 4, false).contains('\x1b'));
}

#[test]
fn huge_top_does_not_generate_empty_rows_proportional_to_request() {
    let mut r = report();
    r.files.clear();
    r.directories.clear();
    let output = render(&r, 120, usize::MAX, false);
    assert!(output.contains("Empty"));
    assert!(output.lines().count() < 30);
    assert!(output.len() < 5000);
}

#[test]
fn partial_totals_and_links_are_visible_and_color_is_optional() {
    let mut r = report();
    r.partial = true;
    r.skipped = 2;
    r.files[0].kind = EntryKind::Link;
    let plain = render(&r, 120, 4, false);
    assert!(plain.contains("PARTIAL TOTAL"));
    assert!(plain.contains("incomplete"));
    assert!(plain.contains("[LINK]"));
    assert!(!plain.contains('\x1b'));
    assert!(render(&r, 120, 4, true).contains("\x1b["));
}

#[test]
fn emoji_presentation_sequences_do_not_overflow_clipped_panels() {
    let mut r = report();
    r.files[0].name = "❤️".repeat(40).into();
    for width in [20, 24, 48, 90] {
        let output = render(&r, width, 4, false);
        assert!(
            output.lines().all(|line| line.width() <= width),
            "width {width}: {output}"
        );
    }
}

#[test]
fn stacked_empty_panel_padding_is_independent_of_the_other_panel() {
    let mut r = report();
    r.directories.clear();
    let output = render(&r, 60, 40, false);
    let folder_panel = output.split("FILES 40 / 45").next().unwrap();
    // Ten bounded padding rows plus the new column header and separators.
    assert!(folder_panel.lines().count() <= 20);
    assert!(output.contains("file-39"));
}

#[test]
fn all_unicode_format_controls_are_escaped() {
    for control in ['\u{00ad}', '\u{180e}', '\u{e0001}', '\u{e0061}'] {
        let name = format!("name{control}end");
        assert!(!safe(OsStr::new(&name)).contains(control));
    }
}

#[test]
fn shares_use_the_full_total_even_when_top_hides_entries() {
    let mut r = report();
    r.files[0].size = 524_288;
    r.directories[0].size = 262_144;
    for top in [1, 4] {
        let out = render(&r, 160, top, false);
        let row = out
            .lines()
            .find(|line| line.contains("folder-00/"))
            .unwrap();
        assert!(row.contains("01"));
        assert!(row.contains("25.0%"));
        assert!(row.contains("50.0%"));
        assert!(out.contains(&format!("FILES {top} / 45")));
        assert!(out.contains("% uses full directory total"));
    }
}

#[test]
fn zero_and_partial_totals_do_not_claim_complete_percentages() {
    let mut r = report();
    r.total = 0;
    r.files[0].size = 0;
    let out = render(&r, 160, 1, false);
    let row = out.lines().find(|line| line.contains("file-00")).unwrap();
    assert!(row.contains('—'));
    assert!(!row.contains("NaN") && !row.contains("inf"));
    r.partial = true;
    r.total = 65_536;
    r.files[0].size = 32_768;
    let out = render(&r, 160, 1, false);
    assert!(out.contains("% KNOWN"));
    assert!(!out.contains("% TOTAL"));
    assert!(out.contains("50.0%"));
    assert!(out.contains("% uses partial total"));
    assert!(out.contains("PARTIAL TOTAL"));
}

#[test]
fn shortened_names_keep_extensions_and_link_markers() {
    let mut r = report();
    r.files[0].name = format!(
        "{}.tar.zst",
        "very-long-archive-name-with-many-parts".repeat(3)
    )
    .into();
    r.files[1].name = "very-long-symlink-name-with-many-parts".into();
    r.files[1].kind = EntryKind::Link;
    for width in [48, 100, 160] {
        let out = render(&r, width, 2, false);
        assert!(out.contains(".tar.zst"), "{width}: {out}");
        assert!(out.contains("[LINK]"), "{width}: {out}");
        assert!(out.contains('…'), "{width}: {out}");
    }
}

fn strip_colors(value: &str) -> String {
    let mut plain = String::new();
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            assert_eq!(chars.next(), Some('['));
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
                assert!(c.is_ascii_digit() || c == ';');
            }
        } else {
            plain.push(c);
        }
    }
    plain
}

#[test]
fn ansi_does_not_change_geometry_and_ranks_stop_at_99() {
    let mut r = report();
    r.files[0].name = "漢字e\u{301}❤️".repeat(15).into();
    r.files.extend((45..105).map(|i| Entry {
        name: format!("file-{i:03}").into(),
        size: u64::MAX,
        kind: EntryKind::File,
    }));
    r.total = u64::MAX;
    for width in [1, 10, 20, 31, 32, 41, 42, 48, 89, 90, 120, 160] {
        let plain = render(&r, width, 105, false);
        let colored = render(&r, width, 105, true);
        assert_eq!(strip_colors(&colored), plain, "width {width}");
        assert!(
            plain.lines().all(|line| line.width() <= width),
            "width {width}"
        );
        if width >= 42 {
            let row = plain
                .lines()
                .find(|line| line.contains("file-098"))
                .unwrap();
            assert!(row.contains("99") && row.contains("100.0%"));
            assert!(!plain.contains("file-099"));
        }
    }
}

fn borders(line: &str) -> Vec<usize> {
    line.char_indices()
        .filter(|(_, c)| *c == '│')
        .map(|(i, _)| line[..i].width())
        .collect()
}

#[test]
fn four_digit_sizes_keep_numeric_columns_and_borders_aligned() {
    let mut r = report();
    r.files[0].size = 1_028_096;
    r.directories[0].size = 1_028_096;
    r.total = 2_056_192;
    for width in [48, 90, 160] {
        let out = render(&r, width, 1, false);
        let columns = borders(out.lines().find(|line| line.contains("NAME")).unwrap());
        for row in out.lines().filter(|line| line.contains("1004.0 KiB")) {
            assert!(row.contains("50.0%"));
            assert_eq!(borders(row), columns, "width {width}: {row}");
            assert!(row.ends_with('│'));
        }
        assert!(out.contains("1004.0 KiB"));
    }
}

#[test]
fn ligatures_keep_borders_and_percentages_even_when_names_are_clipped() {
    let mut r = report();
    r.files[0].name = "لا".repeat(40).into();
    r.directories[0].name = "لا".repeat(40).into();
    for width in [48, 90, 160] {
        let out = render(&r, width, 1, false);
        let columns = borders(out.lines().find(|line| line.contains("NAME")).unwrap());
        for row in out.lines().filter(|line| line.contains("لا")) {
            assert_eq!(borders(row), columns, "width {width}: {row}");
            assert!(row.contains('%') && row.ends_with('│'));
        }
        assert!(out.contains("لا"));
    }
}

#[test]
fn clipped_symlink_preserves_compound_extension_and_marker_together() {
    let mut r = report();
    r.files[0].name = "long-symlink-with-filename.tar.zst".into();
    r.files[0].kind = EntryKind::Link;
    let out = render(&r, 48, 1, false);
    assert!(out.contains(".tar.zst [LINK]"), "{out}");
    assert!(out.contains('…'));
}

fn numeric_colors(output: &str, name: &str) -> Vec<[u8; 3]> {
    let row = output.lines().find(|line| line.contains(name)).unwrap();
    row.split("\x1b[")
        .filter_map(|span| {
            let (code, text) = span.split_once('m')?;
            let visible = text.split('\x1b').next()?.trim();
            if !visible.ends_with('B') && !visible.ends_with('%') {
                return None;
            }
            let codes: Vec<u8> = code.split(';').map(|c| c.parse().unwrap()).collect();
            let start = codes.windows(2).position(|pair| pair == [38, 2])?;
            Some([codes[start + 2], codes[start + 3], codes[start + 4]])
        })
        .collect()
}

fn heat_report() -> Report {
    Report {
        path: "/example".into(),
        total: 20_000,
        partial: false,
        skipped: 0,
        directories: [2000, 1800, 1500, 1100, 800, 200, 0]
            .into_iter()
            .enumerate()
            .map(|(i, size)| Entry {
                name: format!("folder-{i}").into(),
                size,
                kind: EntryKind::Folder,
            })
            .collect(),
        files: [1000, 900, 750, 550, 400, 100, 0]
            .into_iter()
            .enumerate()
            .map(|(i, size)| Entry {
                name: format!("file-{i}").into(),
                size,
                kind: EntryKind::File,
            })
            .collect(),
    }
}

#[test]
fn heat_uses_relative_size_with_gradual_colors_and_normal_small_entries() {
    let r = heat_report();
    let out = render(&r, 80, 99, true);
    let hottest = numeric_colors(&out, "file-0");
    assert_eq!(hottest.len(), 2);
    assert_eq!(hottest[0], hottest[1]);
    let [red, green, blue] = hottest[0];
    assert!(red > 230 && green < 100 && blue < 100);
    let warm = numeric_colors(&out, "file-3")[0];
    assert!(warm[0] > 200 && warm[1] > 170 && warm[2] < 140);
    assert_ne!(hottest, numeric_colors(&out, "file-1"));
    assert_ne!(
        numeric_colors(&out, "file-1"),
        numeric_colors(&out, "file-2")
    );
    assert_ne!(
        numeric_colors(&out, "file-3"),
        numeric_colors(&out, "file-4")
    );
    for i in 0..7 {
        assert_eq!(
            numeric_colors(&out, &format!("file-{i}")),
            numeric_colors(&out, &format!("folder-{i}"))
        );
    }
    let small_top = render(&r, 80, 3, true);
    let normal = numeric_colors(&small_top, "file-0");
    for i in [5, 6] {
        assert_eq!(numeric_colors(&out, &format!("file-{i}")), normal);
    }
    // Removing the ANSI colors must leave exactly the same report and percentages.
    assert_eq!(strip_colors(&out), render(&r, 80, 99, false));
}

#[test]
fn heat_is_stable_across_top_counts_and_zero_totals() {
    let mut r = heat_report();
    let four = render(&r, 80, 4, true);
    let many = render(&r, 80, 99, true);
    for i in 0..4 {
        assert_eq!(
            numeric_colors(&four, &format!("file-{i}")),
            numeric_colors(&many, &format!("file-{i}"))
        );
    }
    let normal = numeric_colors(&render(&r, 80, 3, true), "file-0");
    for entry in r.files.iter_mut().chain(r.directories.iter_mut()) {
        entry.size = 0;
    }
    r.total = 0;
    let out = render(&r, 80, 99, true);
    let zero = numeric_colors(&out, "file-0");
    assert_eq!(zero, vec![normal[0]]); // The undefined percentage is a dash, not a number.
    assert!(!out.contains("NaN"));
}
