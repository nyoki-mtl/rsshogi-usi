use std::fs;
use std::path::{Path, PathBuf};

use rsshogi_usi::{
    assert_invalid_transcript, assert_valid_transcript, format_command, parse_line,
    parse_line_strict, validate_portable_command, UsiCommand,
};

#[test]
fn valid_transcript_roundtrips_to_canonical_format() {
    for path in collect_transcript_files("valid") {
        let content = fs::read_to_string(&path).expect("valid transcript should exist");
        assert_valid_transcript(&content, parse_line, format_command)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

#[test]
fn invalid_transcript_reports_parse_errors() {
    for path in collect_transcript_files("invalid") {
        let content = fs::read_to_string(&path).expect("invalid transcript should exist");
        assert_invalid_transcript(&content, parse_line)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

#[test]
fn strict_valid_transcript_accepts_only_canonical_lines() {
    for path in collect_transcript_files("strict-valid") {
        let content = fs::read_to_string(&path).expect("strict-valid transcript should exist");
        assert_valid_transcript(&content, parse_line_strict, format_command)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

#[test]
fn strict_invalid_transcript_rejects_noncanonical_lines() {
    for path in collect_transcript_files("strict-invalid") {
        let content = fs::read_to_string(&path).expect("strict-invalid transcript should exist");
        assert_invalid_transcript(&content, parse_line_strict)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

#[test]
fn portable_valid_transcript_accepts_canonical_portable_lines() {
    for path in collect_transcript_files("portable-valid") {
        let content = fs::read_to_string(&path).expect("portable-valid transcript should exist");
        assert_valid_transcript(&content, parse_line_portable, format_command)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

#[test]
fn portable_invalid_transcript_rejects_nonportable_lines() {
    for path in collect_transcript_files("portable-invalid") {
        let content = fs::read_to_string(&path).expect("portable-invalid transcript should exist");
        assert_invalid_transcript(&content, parse_line_portable)
            .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
    }
}

fn collect_transcript_files(kind: &str) -> Vec<PathBuf> {
    let root = Path::new("tests/transcripts").join(kind);
    let mut paths = fs::read_dir(&root)
        .unwrap_or_else(|err| panic!("{} should exist: {}", root.display(), err))
        .map(|entry| entry.expect("transcript entry should be readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn parse_line_portable(line: &str) -> Result<UsiCommand, String> {
    let command = parse_line_strict(line).map_err(|err| err.to_string())?;
    validate_portable_command(&command).map_err(|err| err.to_string())?;
    Ok(command)
}
