//! Architecture tests gating structural constraints using `arch-lint` and LOC bounds.

arch_lint::check!();

const MAX_PROD_LOC: usize = 200;

const ALLOWLIST: &[&str] = &[];

/// Opening braces minus closing braces on `line`.
fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |depth: i32, c| match c {
        '{' => depth.saturating_add(1),
        '}' => depth.saturating_sub(1),
        _ => depth,
    })
}

fn count_prod_lines(path: &std::path::Path) -> usize {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    let mut in_test = false;
    let mut brace_depth: i32 = 0;
    let mut prod_lines: usize = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[cfg(test)]") {
            in_test = true;
            brace_depth = brace_delta(line);
            continue;
        }

        if in_test {
            brace_depth = brace_depth.saturating_add(brace_delta(line));
            if brace_depth <= 0 && (line.contains('}') || line.contains(';')) {
                in_test = false;
            }
            continue;
        }

        prod_lines = prod_lines.saturating_add(1);
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

/// Whether `line` is an attribute that lowers a lint for the code under it.
fn is_lint_suppression(line: &str) -> bool {
    let line = line.trim_start();
    (line.starts_with("#[") || line.starts_with("#!["))
        && (line.contains("allow(") || line.contains("expect("))
}

#[test]
fn suppression_scan_recognises_every_attribute_form() {
    for suppression in [
        "#[allow(dead_code)]",
        "    #![allow(clippy::unwrap_used)]",
        "#[expect(unused, reason = \"x\")]",
        "#![expect(clippy::panic)]",
        "#[cfg_attr(test, allow(clippy::too_many_lines))]",
    ] {
        assert!(is_lint_suppression(suppression), "{suppression}");
    }
    for ordinary in [
        "#[test]",
        "#[should_panic(expected = \"boom\")]",
        "let value = result.expect(\"present\");",
        "// #[allow(dead_code)] is not permitted here",
    ] {
        assert!(!is_lint_suppression(ordinary), "{ordinary}");
    }
}

/// Zero lint exceptions: a rule that cannot be met is changed in `Cargo.toml`
/// for everyone, never silenced where it fires.
#[test]
fn no_source_file_suppresses_a_lint() {
    let mut files = Vec::new();
    for root in ["src", "tests", "benches", "crates"] {
        collect_rs_files(std::path::Path::new(root), &mut files);
    }
    assert!(!files.is_empty(), "the scan found no source files");

    let mut suppressions = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(&file).expect("Failed to read file");
        for (index, line) in content.lines().enumerate() {
            if is_lint_suppression(line) {
                let number = index.saturating_add(1);
                suppressions.push(format!("{}:{number}: {}", file.display(), line.trim()));
            }
        }
    }

    assert!(
        suppressions.is_empty(),
        "lint suppressions are not allowed:\n{}",
        suppressions.join("\n")
    );
}

fn lints_table(manifest: &str) -> toml::Table {
    let content = std::fs::read_to_string(manifest).expect("Failed to read manifest");
    let manifest: toml::Table = content.parse().expect("Manifest is valid TOML");
    manifest
        .get("lints")
        .and_then(toml::Value::as_table)
        .cloned()
        .expect("Manifest has a [lints] table")
}

/// The member crate holding the unsafe code follows every rule of the main
/// crate except the two that are the reason it exists.
#[test]
fn member_crate_lints_differ_only_where_unsafe_requires() {
    let mut expected = lints_table("Cargo.toml");
    let rust = expected
        .get_mut("rust")
        .and_then(toml::Value::as_table_mut)
        .expect("[lints.rust]");
    assert_eq!(
        rust.remove("unsafe_code"),
        Some(toml::Value::from("forbid")),
        "the main crate must forbid unsafe code"
    );
    expected
        .get_mut("clippy")
        .and_then(toml::Value::as_table_mut)
        .expect("[lints.clippy]")
        .insert(
            "non_send_fields_in_send_ty".to_string(),
            toml::Value::from("allow"),
        );

    assert_eq!(lints_table("crates/cranky-unsafe/Cargo.toml"), expected);
}
