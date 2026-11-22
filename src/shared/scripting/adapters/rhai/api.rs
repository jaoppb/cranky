//! Registers what a Rhai module script can call, such as the `ui` elements and actions, `sys`,
//! `exec` and `assert`.
use super::dsl_actions::{register_rhai_ui_actions, run_shell_command};
use super::dsl_elements::register_rhai_ui_elements;
use super::dsl_sys::register_rhai_sys;
use rhai::{Dynamic, Engine};

/// Registers the `cranky`, `ui`, `signals`, `config`, and `sys` APIs in a Rhai `Engine`.
pub(crate) fn register_rhai_cranky_api(engine: &mut Engine) {
    register_rhai_ui_elements(engine);
    register_rhai_ui_actions(engine);
    register_rhai_sys(engine);

    // Standalone exec for backwards compatibility
    engine.register_fn("exec", |cmd: String| run_shell_command(&cmd));

    register_rhai_assertions(engine);
}

/// Registers `assert(cond)` and `assert(cond, msg)`, which fail the script when `cond` is false.
fn register_rhai_assertions(engine: &mut Engine) {
    engine.register_fn(
        "assert",
        |cond: bool| -> Result<(), Box<rhai::EvalAltResult>> {
            if !cond {
                return Err(assertion_failed("Assertion failed in Rhai script".to_string()).into());
            }
            Ok(())
        },
    );
    engine.register_fn(
        "assert",
        |cond: bool, msg: String| -> Result<(), Box<rhai::EvalAltResult>> {
            if !cond {
                return Err(
                    assertion_failed(format!("Assertion failed in Rhai script: {msg}")).into(),
                );
            }
            Ok(())
        },
    );
}

/// The runtime error a failed assertion raises, carrying `message`.
fn assertion_failed(message: String) -> rhai::EvalAltResult {
    rhai::EvalAltResult::ErrorRuntime(Dynamic::from(message), rhai::Position::NONE)
}
