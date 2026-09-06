//! Architecture tests gating structural constraints using `arch-lint` and LOC bounds.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

arch_lint::check!();

const MAX_PROD_LOC: usize = 200;

const ALLOWLIST: &[&str] = &[
    "src/shared/wayland/adapters/wayland.rs",
];

fn count_prod_lines(path: &std::path::Path) -> usize {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    let mut in_test = false;
    let mut brace_depth: i32 = 0;
    let mut prod_lines = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            in_test = true;
            let opens = line.chars().filter(|&c| c == '{').count() as i32;
            let closes = line.chars().filter(|&c| c == '}').count() as i32;
            brace_depth = opens - closes;
            continue;
        }

        if in_test {
            let opens = line.chars().filter(|&c| c == '{').count() as i32;
            let closes = line.chars().filter(|&c| c == '}').count() as i32;
            brace_depth += opens - closes;
            if brace_depth <= 0 && (line.contains('}') || line.contains(';')) {
                in_test = false;
            }
            continue;
        }

        prod_lines += 1;
    }

    prod_lines
}

fn collect_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    if !dir.exists() {
        return;
    }
    for entry in std::fs::read_dir(dir).expect("Failed to read dir") {
        let entry = entry.expect("Valid dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn verify_file_loc_limits() {
    let mut files = Vec::new();
    collect_rs_files(std::path::Path::new("src/features"), &mut files);
    collect_rs_files(std::path::Path::new("src/shared"), &mut files);

    let mut violations = Vec::new();

    for file in files {
        let path_str = file.to_str().expect("Valid UTF-8 path");
        let normalized = path_str.replace('\\', "/");
        let loc = count_prod_lines(&file);

        if loc > MAX_PROD_LOC {
            let is_allowed = ALLOWLIST.iter().any(|&allowed| allowed == normalized);
            if !is_allowed {
                violations.push(format!("{normalized}: {loc} LOC (limit: {MAX_PROD_LOC})"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "The following files exceed the {MAX_PROD_LOC} production LOC limit:\n{}",
        violations.join("\n")
    );
}

