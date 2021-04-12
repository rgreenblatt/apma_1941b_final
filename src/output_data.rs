//! TODO: consider removing this module
use anyhow::Result;
use std::{fs::File, path::Path};

pub fn csv_writer(path: &Path) -> Result<csv::Writer<File>> {
  let file = File::create(path)?;
  let out = csv::Writer::from_writer(file);
  Ok(out)
}

pub fn csv_reader(path: &Path) -> Result<csv::Reader<File>> {
  let file = File::open(path)?;
  let out = csv::Reader::from_reader(file);
  Ok(out)
}
