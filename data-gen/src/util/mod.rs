use std::{
  error::Error,
  fs,
  path::{Path, PathBuf},
};

/// Loads all the files with the given name from the data dir. `p` should be the
/// OUT_DIR of the program, and name should be something like `blocks.json`.
pub fn load_versions(p: &Path, name: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
  let dir = p.join("prismarine-data/data/pc");

  let mut versions = vec![];
  for ver in fs::read_dir(dir)? {
    let path = ver?.path();
    let named_path = path.join(name);
    if named_path.exists() {
      let fname = path.file_name().unwrap().to_str().unwrap();
      // We use 1.13.2 instead of 1.13, and 1.7 is different so I haven't bothered to
      // parse it.
      if fname == "1.13" || fname == "1.7" || fname.chars().any(|c| c.is_ascii_lowercase()) {
        continue;
      }
      versions.push(named_path);
    }
  }
  Ok(versions)
}
