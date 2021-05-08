use std::{env, error::Error, fs, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("block/kind.rs");
  fs::create_dir_all(dest_path.parent().unwrap())?;
  fs::write(
    &dest_path,
    "
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub enum Kind {
      Air,
      Grass,
      Dirt
    }",
  )?;
  println!("cargo:rerun-if-changed=build.rs");
  Ok(())
}
