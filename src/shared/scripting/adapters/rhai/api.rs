use super::dsl_actions::register_rhai_ui_actions;
use super::dsl_elements::register_rhai_ui_elements;
use super::dsl_sys::register_rhai_sys;
use rhai::{Dynamic, Engine};

/// Registers the `cranky`, `ui`, `signals`, `config`, and `sys` APIs in a Rhai `Engine`.
pub fn register_rhai_cranky_api(engine: &mut Engine) {
    register_rhai_ui_elements(engine);
    register_rhai_ui_actions(engine);
    register_rhai_sys(engine);

    // Standalone exec for backwards compatibility
    engine.register_fn("exec", |cmd: String| {
        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
    });

    // Script assertions
    engine.register_fn(
        "assert",
        |cond: bool| -> Result<(), Box<rhai::EvalAltResult>> {
            if !cond {
                return Err(rhai::EvalAltResult::ErrorRuntime(
                    Dynamic::from("Assertion failed in Rhai script"),
                    rhai::Position::NONE,
                )
                .into());
            }
            Ok(())
        },
    );
    engine.register_fn(
        "assert",
        |cond: bool, msg: String| -> Result<(), Box<rhai::EvalAltResult>> {
            if !cond {
                return Err(rhai::EvalAltResult::ErrorRuntime(
                    Dynamic::from(format!("Assertion failed in Rhai script: {msg}")),
                    rhai::Position::NONE,
                )
                .into());
            }
            Ok(())
        },
    );
}
