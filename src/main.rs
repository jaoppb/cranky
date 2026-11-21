//! The `cranky` binary: runs the bar until it exits.

fn main() -> Result<(), cranky::app::BootError> {
    cranky::app::run()
}
