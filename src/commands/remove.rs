use anyhow::Result;

use crate::napm::Napm;

pub fn run(pkgs: &[&str], deep: bool) -> Result<()> {
    let mut napm = Napm::new()?;
    napm.remove(pkgs, deep)
}
