use std::{fs, path::Path};

fn main() {
  // Create config directories in the crate being built
  let config_dir = Path::new("config/retrievers");
  fs::create_dir_all(config_dir).unwrap();

  // Navigate from crate root to repo root config
  let source_dir = Path::new("../../config/retrievers");

  let files = ["arxiv.toml", "doi.toml", "iacr.toml"];
  for file in files {
    let source = source_dir.join(file);
    let dest = config_dir.join(file);

    fs::copy(&source, &dest)
      .unwrap_or_else(|_| panic!("Failed to copy {} to {}", source.display(), dest.display()));
  }

  // Tell cargo to rerun if configs change
  println!("cargo:rerun-if-changed=../../config/retrievers");
}
