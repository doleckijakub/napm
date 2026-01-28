use crate::error::Result;
use crate::napm::Napm;

pub fn run(napm: &mut Napm, pkg_name: &str, with_dirs: bool) -> Result<()> {
    for f in napm.files(pkg_name, with_dirs)? {
        println!("{}", f);
    }

    Ok(())
}
