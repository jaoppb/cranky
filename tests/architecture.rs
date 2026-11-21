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

/// Every attribute in `source` that lowers a lint, with the line it starts on.
///
/// Reads whole attributes, not lines, so one split over several lines or
/// written after other tokens is still found. Comment lines are ignored.
fn lint_suppressions(source: &str) -> Vec<(usize, String)> {
    let code = source
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("//") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut found = Vec::new();
    for (start, _) in code.match_indices('#') {
        let rest = code.get(start.saturating_add(1)..).unwrap_or_default();
        let rest = rest.strip_prefix('!').unwrap_or(rest);
        if !rest.starts_with('[') {
            continue;
        }
        let mut depth: usize = 0;
        let mut attribute = String::new();
        for character in rest.chars() {
            match character {
                '[' => depth = depth.saturating_add(1),
                ']' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if !character.is_whitespace() {
                attribute.push(character);
            }
            if depth == 0 {
                break;
            }
        }
        if attribute.contains("allow(") || attribute.contains("expect(") {
            let line = code
                .get(..start)
                .unwrap_or_default()
                .matches('\n')
                .count()
                .saturating_add(1);
            found.push((line, attribute));
        }
    }
    found
}

#[test]
fn suppression_scan_finds_single_line_multi_line_and_inline_attributes() {
    // Built from pieces so that this file does not contain an attribute the
    // scan would report.
    let hash = '#';
    for suppression in [
        format!("{hash}[allow(dead_code)]"),
        format!("    {hash}![allow(clippy::unwrap_used)]"),
        format!("{hash}[expect(unused, reason = \"x\")]"),
        format!("{hash}![expect(clippy::panic)]"),
        format!("{hash}[cfg_attr(test, allow(clippy::too_many_lines))]"),
        format!(
            "{hash}[cfg_attr(\n    test,\n    expect(clippy::too_many_lines, reason = \"x\")\n)]"
        ),
        format!("{hash}[allow(\n    dead_code\n)]"),
        format!("fn f({hash}[expect(unused_variables)] x: u8) {{}}"),
    ] {
        assert_eq!(lint_suppressions(&suppression).len(), 1, "{suppression}");
    }
    for ordinary in [
        format!("{hash}[test]"),
        format!("{hash}[should_panic(expected = \"boom\")]"),
        "let value = result.expect(\"present\");".to_string(),
        format!("// {hash}[allow(dead_code)] is not permitted here"),
        format!("{hash}[derive(Debug)]\nstruct Allow(u8);"),
    ] {
        assert_eq!(lint_suppressions(&ordinary), [], "{ordinary}");
    }
}

#[test]
fn suppression_scan_reports_the_line_an_attribute_starts_on() {
    let hash = '#';
    let source = format!("fn a() {{}}\n\n{hash}[allow(\n    dead_code\n)]\nfn b() {{}}\n");
    assert_eq!(
        lint_suppressions(&source),
        [(3, "[allow(dead_code)]".to_string())]
    );
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
        for (line, attribute) in lint_suppressions(&content) {
            suppressions.push(format!("{}:{line}: #{attribute}", file.display()));
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

/// The member crate that tells the time has no reason to differ: it follows
/// every rule of the main crate, `unsafe_code` included.
#[test]
fn time_crate_lints_equal_the_main_crate() {
    assert_eq!(
        lints_table("crates/cranky-time/Cargo.toml"),
        lints_table("Cargo.toml")
    );
}

/// What an Arch `PKGBUILD` assigns to `pkgver`.
fn pkgver(pkgbuild: &str) -> String {
    pkgbuild
        .lines()
        .find_map(|line| line.strip_prefix("pkgver="))
        .expect("the PKGBUILD sets pkgver")
        .to_string()
}

/// A release is cut from one version: the tag must equal the workspace version
/// (`.github/workflows/release.yml` checks that), and the Arch packages are
/// published from the `PKGBUILD` files as they are at the tag.
/// `just bump-version` writes all of them; this fails when one was missed.
/// `Cargo.lock` is not read here: cargo brings it up to date before a test runs.
#[test]
fn version_files_agree() {
    let read = |path: &str| std::fs::read_to_string(path).expect("Failed to read file");
    let manifest: toml::Table = read("Cargo.toml").parse().expect("Manifest is valid TOML");
    let version = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .expect("Manifest has [workspace.package].version");

    assert_eq!(pkgver(&read("contrib/arch/PKGBUILD")), version);
    assert_eq!(pkgver(&read("contrib/arch/PKGBUILD-bin")), version);

    // The git package computes its version from the tags: the literal is a
    // placeholder, and `pkgver()` falls back to it when the clone has no tag.
    let git_pkgbuild = read("contrib/arch/PKGBUILD-git");
    assert_eq!(pkgver(&git_pkgbuild), format!("{version}.r0.g0000000"));
    let fallback = format!("printf \"{version}.r%s.%s\"");
    assert!(
        git_pkgbuild.contains(&fallback),
        "PKGBUILD-git's pkgver() must fall back to {fallback}"
    );
}
