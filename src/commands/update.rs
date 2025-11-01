use anyhow::Result;

use crate::napm::Napm;

pub fn run() -> Result<()> {
    let mut napm = Napm::new()?;

    if let Some(r) = napm.update() {
        r
    } else {
        println!("nothing to do");

        Ok(())
    }
}
