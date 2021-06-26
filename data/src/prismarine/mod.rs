use std::{fs, io, path::Path, process::Command, str};

const REMOTE: &str = "https://github.com/PrismarineJS/minecraft-data.git";
const COMMIT: &str = "140e1a64aa1f54d56d3e0a820e18e4432d6133ef";

fn run_git(path: &Path, args: &[&str]) -> String {
  let out =
    Command::new("git").current_dir(path).args(args).output().expect("failed to execute git");
  dbg!(&out);
  let stdout = str::from_utf8(&out.stdout).unwrap();
  println!("{}", stdout);
  assert_eq!(0, out.status.code().unwrap()); // Make sure this didn't fail
  return stdout.into();
}

pub fn clone(dir: &Path) -> io::Result<()> {
  let path = dir.join("prismarine-data");
  dbg!(&path);
  if !path.exists() {
    fs::create_dir_all(&path)?;
  }
  let out = run_git(&path, &["rev-parse", "--show-toplevel"]);
  if !out.ends_with("prismarine-data") {
    // Means that the root repo is not prismarine data, so we need to clone
    if fs::read_dir(&path)?.next().is_some() {
      fs::remove_dir_all(&path)?;
      fs::create_dir_all(&path)?;
    }
    println!("cloning repository...");
    run_git(&path, &["init"]);
    run_git(&path, &["remote", "add", "origin", REMOTE]);
    run_git(&path, &["fetch", "origin", COMMIT, "--depth=1"]);
  }
  run_git(&path, &["checkout", COMMIT]);
  Ok(())
}
