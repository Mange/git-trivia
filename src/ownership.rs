use std::cmp::Ordering;

use indicatif::{ProgressBar, ProgressStyle};
use git2::{Commit, BlameOptions, BlameHunk};

use super::errors::*;
use super::{TreeWalker, Context};
use person::{Person, CombinedTracking};

#[derive(Debug)]
pub struct OwnershipStatistics<'context> {
    pub total_lines: u32,
    pub combined_tracking: CombinedTracking<'context, OwnershipScore>,
}

#[derive(Debug, Serialize)]
pub struct ComputedOwnership {
    pub total_lines_owned: u32,
    pub fraction_owned: f32,
}

impl ComputedOwnership {
    pub fn percent_owned(&self) -> f32 {
        self.fraction_owned * 100.0
    }
}

impl PartialEq<ComputedOwnership> for ComputedOwnership {
    fn eq(&self, other: &ComputedOwnership) -> bool {
        self.total_lines_owned.eq(&other.total_lines_owned)
    }
}

impl Eq for ComputedOwnership {}

impl PartialOrd for ComputedOwnership {
    fn partial_cmp(&self, other: &ComputedOwnership) -> Option<Ordering> {
        self.total_lines_owned.partial_cmp(&other.total_lines_owned)
    }
}

impl Ord for ComputedOwnership {
    fn cmp(&self, other: &ComputedOwnership) -> Ordering {
        self.total_lines_owned.cmp(&other.total_lines_owned)
    }
}

impl<'context> OwnershipStatistics<'context> {
    pub fn from_tracking(
        owners: CombinedTracking<'context, OwnershipScore>,
    ) -> OwnershipStatistics<'context> {
        let total_lines = owners
            .people_iter()
            .map(|(_, score)| score.total_lines_owned)
            .sum();
        OwnershipStatistics {
            total_lines: total_lines,
            combined_tracking: owners,
        }
    }

    pub fn total_lines(&self) -> u32 {
        self.total_lines
    }

    pub fn people_toplist(&self) -> Vec<(&Person, ComputedOwnership)> {
        let mut toplist: Vec<_> = self.combined_tracking
            .people_iter()
            .map(|(person, score)| (*person, self.compute_ownership(score)))
            .collect();
        toplist.sort_by(|a, b| b.1.cmp(&a.1)); // Note: Reverse sort
        toplist
    }

    pub fn teams_toplist(&self) -> Vec<(Option<&str>, ComputedOwnership)> {
        let mut toplist: Vec<_> = self.combined_tracking
            .team_iter()
            .map(|(team_name, score)| {
                (team_name, self.compute_ownership(score))
            })
            .collect();
        toplist.sort_by(|a, b| b.1.cmp(&a.1)); // Note: Reverse sort
        toplist
    }

    fn compute_ownership(&self, score: &OwnershipScore) -> ComputedOwnership {
        ComputedOwnership {
            total_lines_owned: score.total_lines_owned,
            fraction_owned: (score.total_lines_owned as f32 / self.total_lines as f32),
        }
    }
}

#[derive(Debug)]
pub struct OwnershipScore {
    pub total_lines_owned: u32,
}

impl Default for OwnershipScore {
    fn default() -> OwnershipScore {
        OwnershipScore { total_lines_owned: 0 }
    }
}

impl OwnershipScore {
    fn add_hunk(&mut self, hunk: &BlameHunk) {
        self.total_lines_owned += hunk.lines_in_hunk() as u32;
    }
}

pub fn calculate<'context>(
    context: &'context Context,
    commit: &Commit,
) -> Result<OwnershipStatistics<'context>> {
    let people_db = context.people_db();
    let repo = context.repo();

    let mut owners: CombinedTracking<OwnershipScore> = CombinedTracking::new();

    let mut blame_options = BlameOptions::default();
    blame_options.newest_commit(commit.id());

    let total_files = TreeWalker::new(repo, commit.tree()?).count();
    let progress = ProgressBar::new(total_files as u64);
    progress.set_style(ProgressStyle::default_bar().template(
        "[{eta}] {bar:40.cyan/blue} {pos}/{len} - {wide_msg}",
    ));

    for entry in TreeWalker::new(repo, commit.tree()?) {
        progress.set_message(&format!("Blaming {}", entry.path().display()));
        if entry.is_file() && !entry.blob(repo).unwrap().is_binary() {
            let blame = repo.blame_file(entry.path(), Some(&mut blame_options))?;
            for hunk in blame.iter() {
                let person = people_db.find_by_signature(hunk.orig_signature())?;
                owners.track_person(person, |score| score.add_hunk(&hunk));
            }
        }
        progress.inc(1);
    }

    progress.set_message("");
    progress.finish();

    Ok(OwnershipStatistics::from_tracking(owners))
}
