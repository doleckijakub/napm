use anyhow::Result;

use crate::napm::Napm;

pub fn run(pkg: &str) -> Result<()> {
    let napm = Napm::new()?;

    let p = napm.info(pkg)?;

    println!("Name          : {}", p.name);
    println!("Version       : {}", p.version);
    println!("Description   : {}", p.desc);

    Ok(())
}
