#[cfg(test)]
use crate::{github_api, dataset::ContributionInput, Repo, User};
use crate::{dataset::Dataset, ItemType, UserRepoPair};
#[cfg(test)]
use std::{collections::HashSet, iter};

pub type Component = UserRepoPair<Vec<usize>>;
