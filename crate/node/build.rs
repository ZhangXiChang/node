use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    match fl2rust::Generator::default().in_out("./ui/user_interface.fl", "./ui/user_interface.rs") {
        Ok(_) => (),
        Err(err) => return Err(anyhow!(err.to_string())),
    };
    Ok(())
}
