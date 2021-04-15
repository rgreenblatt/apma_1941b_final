use indicatif::{ProgressBar, ProgressStyle};

#[must_use]
pub fn get_bar(count: Option<u64>, draw_delta: u64) -> ProgressBar {
  let bar = ProgressBar::new(count.unwrap_or(!0));
  let template = if count.is_some() {
    "[{elapsed_precise}] {bar} {pos:>7} / {len:>7} {eta_precise} {per_sec}"
  } else {
    "[{elapsed_precise}] {pos} {per_sec}"
  };
  bar.set_style(ProgressStyle::default_bar().template(template));
  bar.set_draw_delta(draw_delta);
  bar
}
