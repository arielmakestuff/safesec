use std::env::{current_dir, set_current_dir};
use std::io;
use std::path::Path;


// Change directory, run closure, then change directory back to original directory
pub fn cd<P, F>(path: P, block: F) -> io::Result<()>
        where P: AsRef<Path>,
              F: FnOnce(&Path) -> io::Result<()> {
    let old = current_dir()?;
    set_current_dir(path.as_ref())?;
    block(path.as_ref())?;
    set_current_dir(old.as_path())?;
    Ok(())
}
