use std::{fs, io, path::Path, process::Command, str};

pub fn clone(dir: &Path) -> io::Result<()> {
  let path = dir.join("prismarine-data");
  dbg!(&path);
  if !path.exists() {
    fs::create_dir_all(&path)?;
  }
  let out = Command::new("git")
    .current_dir(&path)
    .args(&["status", "--short"])
    .output()
    .expect("failed to execute git");
  let out = str::from_utf8(&out.stdout).unwrap();
  println!("status: {}", out);
  if out.contains("..") {
    // Means that the status was on the root repo, so we need to clone
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
    let out = str::from_utf8(&out.stdout).unwrap();
    println!("{}", out);
  }
  Ok(())
}
