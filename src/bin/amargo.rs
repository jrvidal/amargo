use std::error::Error;
use std::process::Command;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let exit = Command::new("cargo")
        // We don't respect existing RUSTC_WRAPPER
        .env("RUSTC_WRAPPER", "ruugts")
        .env("RUUGTS_WRAPPER", "on")
        .args(std::env::args().skip(1))
        .spawn()?
        .wait()?;
    if !exit.success() {
        std::process::exit(exit.code().unwrap_or(1));
    }
    Ok(())
}
