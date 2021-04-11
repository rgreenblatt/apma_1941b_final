use anyhow::Result;
use std::{
  fs::{self, File},
  path::Path,
};

pub fn output_data_dir() -> Result<&'static Path> {
  let out = Path::new("output_data/");
  fs::create_dir_all(out)?;
  Ok(out)
}

pub fn csv_writer(path: &str) -> Result<csv::Writer<File>> {
  let file = File::create(output_data_dir()?.join(path))?;
  let out = csv::Writer::from_writer(file);
  Ok(out)
}

pub fn csv_reader(path: &str) -> Result<csv::Reader<File>> {
  let file = File::open(output_data_dir()?.join(path))?;
  let out = csv::Reader::from_reader(file);
  Ok(out)
}
