#[cfg(test)]
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate diesel;

pub mod db;
pub mod github_api;

pub use db::models::{Repo, User};
