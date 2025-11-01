use anyhow::Result;

use crate::napm::Napm;

pub fn run() -> Result<()> {
    let napm = Napm::new()?;

    for pkg in napm.list() {
        println!("{} {}", pkg.name, pkg.version);
    }

    Ok(())
}
