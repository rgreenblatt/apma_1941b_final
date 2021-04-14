use flate2::read::GzDecoder;
use serde::Deserialize;
use std::{
  fs::File,
  io::{prelude::*, BufReader},
  marker::PhantomData,
  path::PathBuf,
};

struct CsvItemsIter<T> {
  files: Vec<PathBuf>,
  /// buffer for speed because external iteration is very slow...
  /// really should be using try_fold instead, but that isn't stabilized
  items: Vec<csv::Result<T>>,
  files_index: usize,
  reader: Option<csv::DeserializeRecordsIntoIter<GzDecoder<File>, T>>,
  finished: bool,
  _priv: PhantomData<T>,
}

impl<T> CsvItemsIter<T>
where
  T: for<'a> Deserialize<'a>,
{
  fn load_in_items(&mut self) -> Option<std::io::Result<()>> {
    assert!(self.items.is_empty());
    if self.reader.is_none() {
      let file = File::open(self.files.get(self.files_index)?);
      self.files_index += 1;
      let file = match file {
        Ok(file) => file,
        Err(e) => return Some(Err(e)),
      };
      self.reader =
        Some(csv::Reader::from_reader(GzDecoder::new(file)).into_deserialize());
    }

    let count = 10_000;

    self
      .items
      .extend(self.reader.as_mut().unwrap().into_iter().take(count));

    if self.items.len() < count {
      self.reader = None;
    }

    Some(Ok(()))
  }
}

impl<T> Iterator for CsvItemsIter<T>
where
  T: for<'a> Deserialize<'a>,
{
  type Item = anyhow::Result<T>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.finished {
      return None;
    }
    loop {
      if let Some(item) = self.items.pop() {
        return Some(item.map_err(Into::into));
      }

      if let Err(e) = self.load_in_items()? {
        self.finished = true;
        return Some(Err(e.into()));
      }
    }
  }
}

pub fn csv_items_iter<T>(
  list: PathBuf,
) -> std::io::Result<impl Iterator<Item = anyhow::Result<T>>>
where
  T: for<'a> Deserialize<'a>,
{
  let user_reader = BufReader::new(File::open(list)?);
  let files = user_reader
    .lines()
    .map(|l| l.map(Into::into))
    .collect::<std::result::Result<Vec<_>, _>>()?;

  let out = CsvItemsIter::<T> {
    files,
    items: Vec::new(),
    files_index: 0,
    reader: None,
    finished: false,
    _priv: Default::default(),
  };

  Ok(out)
}
