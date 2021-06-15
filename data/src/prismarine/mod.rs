use std::{fs, io, path::Path, process::Command, str};

fn checkout(path: &Path) {
  // TODO: We want to fetch here if we can't check out.
  let out = Command::new("git")
    .current_dir(path)
    .args(&["checkout", "140e1a64aa1f54d56d3e0a820e18e4432d6133ef"])
    .output()
    .expect("failed to execute git");
  dbg!(&out);
  assert_eq!(0, out.status.code().unwrap()); // Make sure this didn't fail
  let out = str::from_utf8(&out.stdout).unwrap();
  println!("{}", out);
}

pub fn clone(dir: &Path) -> io::Result<()> {
  let path = dir.join("prismarine-data");
  dbg!(&path);
  if !path.exists() {
    fs::create_dir_all(&path)?;
  }
  let out = Command::new("git")
    .current_dir(&path)
    .args(&["rev-parse", "--show-toplevel"])
    .output()
    .expect("failed to execute git");
  dbg!(&out);
  assert_eq!(0, out.status.code().unwrap()); // Make sure this dodn't fail
  let out = str::from_utf8(&out.stdout).unwrap();
  println!("top level: {}", out);
  if !out.ends_with("prismarine-data") {
    // Means that the root repo is not prismarine data, so we need to clone
    if fs::read_dir(&path)?.next().is_some() {
      fs::remove_dir_all(&path)?;
      fs::create_dir_all(&path)?;
    }
    println!("cloning repository...");
    let out = Command::new("git")
      .current_dir(&path)
      .args(&["clone", "https://github.com/PrismarineJS/minecraft-data.git", "."])
      .output()
      .expect("failed to execute git");
    dbg!(&out);
    assert_eq!(0, out.status.code().unwrap()); // Make sure this dodn't fail
    let out = str::from_utf8(&out.stdout).unwrap();
    println!("{}", out);
    checkout(&path);
  } else {
    // We already have a good repo, make sure we are checked out correctly
    checkout(&path);
  }
  Ok(())
}
