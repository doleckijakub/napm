use crate::napm::Napm;
use anyhow::Result;

pub fn run(pkg_names: &[&str]) -> Result<()> {
    let mut napm = Napm::new()?;
    let _ = napm.sync(false)?;
    napm.install(pkg_names)
}
