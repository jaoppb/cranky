//! The command line, which `cargo test` shares with libtest binaries.

use crate::support::{Failure, Outcome};

/// libtest flags followed by a value that is not a case filter.
const WITH_VALUE: [&str; 5] = ["--test-threads", "--format", "--color", "--logfile", "-Z"];

/// libtest flags that change nothing here: output is never captured and
/// cases always run one at a time.
const IGNORED: [&str; 6] = [
    "--nocapture",
    "--no-capture",
    "--show-output",
    "--include-ignored",
    "--quiet",
    "-q",
];

/// Which cases the command line asks for.
#[derive(Default)]
pub(crate) struct Selection {
    filters: Vec<String>,
    skips: Vec<String>,
    exact: bool,
    list: bool,
    only_ignored: bool,
}

impl Selection {
    /// Reads the arguments after the program name. An argument this binary
    /// does not know fails the run: guessing could skip cases silently.
    pub(crate) fn parse(mut args: impl Iterator<Item = String>) -> Outcome<Self> {
        let mut selection = Self::default();
        while let Some(arg) = args.next() {
            let flag = arg.split('=').next().unwrap_or_default();
            match flag {
                "--skip" => selection.skips.push(value_of(&arg, &mut args)?),
                "--exact" => selection.exact = true,
                "--list" => selection.list = true,
                "--ignored" => selection.only_ignored = true,
                _ if WITH_VALUE.contains(&flag) => drop(value_of(&arg, &mut args)?),
                _ if IGNORED.contains(&flag) => {}
                _ if flag.starts_with('-') => {
                    return Err(Failure::new(format!("unsupported argument `{arg}`")));
                }
                _ => selection.filters.push(arg),
            }
        }
        Ok(selection)
    }

    /// Whether `--list` asked for the case names instead of a run.
    pub(crate) const fn list(&self) -> bool {
        self.list
    }

    /// Whether the case called `name` is asked for. No case is ever ignored, so `--ignored` asks for none of them.
    pub(crate) fn accepts(&self, name: &str) -> bool {
        if self.only_ignored {
            return false;
        }
        let matches = |pattern: &String| {
            if self.exact {
                name == pattern
            } else {
                name.contains(pattern.as_str())
            }
        };
        (self.filters.is_empty() || self.filters.iter().any(matches))
            && !self.skips.iter().any(matches)
    }
}

/// The value of `flag`, written as `--flag=value` or as the next argument.
fn value_of(flag: &str, rest: &mut impl Iterator<Item = String>) -> Outcome<String> {
    flag.split_once('=')
        .map(|(_, value)| value.to_string())
        .or_else(|| rest.next())
        .ok_or_else(|| Failure::new(format!("`{flag}` needs a value")))
}
