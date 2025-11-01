use anyhow::Result;

use crate::napm::Napm;

pub fn run(pkg: &str) -> Result<()> {
    let napm = Napm::new()?;

    for f in napm.files(pkg)? {
        println!("{}", f);
    }

    Ok(())
}
