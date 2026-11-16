//! The case list and the runner that replaces libtest.

use crate::sandbox::Sandbox;
use crate::support::{Context, Failure, Outcome, ensure, within_for};
use crate::{dbus, systray, wayland};
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// Deadline of the whole run, on top of the one on every wait.
const RUN: Duration = Duration::from_secs(60);

pub type CaseFuture = Pin<Box<dyn Future<Output = Outcome>>>;

pub struct Case {
    name: &'static str,
    run: fn() -> CaseFuture,
}

impl Case {
    pub const fn new(name: &'static str, run: fn() -> CaseFuture) -> Self {
        Self { name, run }
    }
}

/// Builds a [`Case`] named after the async function that runs it.
macro_rules! case {
    ($module:ident :: $function:ident) => {
        $crate::cases::Case::new(
            concat!(stringify!($module), "::", stringify!($function)),
            || Box::pin($function()),
        )
    };
}
pub(crate) use case;

fn all() -> Vec<Case> {
    let mut cases = dbus::cases();
    cases.extend(systray::cases());
    cases.extend(wayland::cases());
    cases
}

fn runtime() -> Outcome<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build the tokio runtime")
}

/// Parent side: starts the sandbox and runs, each in a process of its own,
/// the cases whose name contains any of `filters` (all of them without
/// filters). A process per case means nothing a case leaves behind, such as
/// a bus name it owns, reaches the next one.
pub fn run_selected(filters: &[String]) -> Outcome {
    let names: Vec<&str> = all()
        .iter()
        .map(|case| case.name)
        .filter(|name| filters.is_empty() || filters.iter().any(|f| name.contains(f.as_str())))
        .collect();
    if names.is_empty() {
        println!("integration: no case matches {filters:?}, nothing to run");
        return Ok(());
    }

    runtime()?.block_on(async {
        let sandbox = Sandbox::start().await?;
        println!("\nrunning {} integration cases", names.len());
        let failed = within_for(RUN, "the cases to finish", run_each(&sandbox, &names)).await;
        sandbox.stop().await;
        let failed = failed?;
        println!();
        ensure(failed.is_empty(), || {
            format!(
                "{} of {} cases failed: {}",
                failed.len(),
                names.len(),
                failed.join(", ")
            )
        })
    })
}

/// Runs and reports each case; returns the names of the ones that failed.
async fn run_each<'a>(sandbox: &Sandbox, names: &[&'a str]) -> Vec<&'a str> {
    let mut failed = Vec::new();
    for name in names {
        let started = Instant::now();
        match sandbox.run_case(name).await {
            Ok(()) => println!("case {name} ... ok ({:?})", started.elapsed()),
            Err(failure) => {
                println!("case {name} ... FAILED\n{failure}");
                failed.push(*name);
            }
        }
    }
    failed
}

/// Child side: runs the case called exactly `name`, inside the sandbox.
pub fn run_one(name: &str) -> Outcome {
    let case = all()
        .into_iter()
        .find(|case| case.name == name)
        .ok_or_else(|| Failure::new(format!("no case is called {name}")))?;
    runtime()?.block_on((case.run)())
}
