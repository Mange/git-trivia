extern crate indicatif;

use indicatif::ProgressBar;
use git2::{Commit, BlameOptions, BlameHunk};

use super::errors::*;
use super::{TreeWalker, Context};
use person::PeopleTracking;

struct OwnershipScore {
    total_lines_owned: u32,
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

pub fn calculate(context: &Context, commit: &Commit) -> Result<()> {
    let people_db = context.people_db();
    let repo = context.repo();

    let mut owners: PeopleTracking<OwnershipScore> = PeopleTracking::new();

    let mut blame_options = BlameOptions::new();
    blame_options.newest_commit(commit.id());

    let total_files = TreeWalker::new(repo, commit.tree()?).count();
    let progress = ProgressBar::new(total_files as u64);

    for entry in TreeWalker::new(repo, commit.tree()?) {
        if entry.is_file() {
            if !entry.blob(repo).unwrap().is_binary() {
                let blame = repo.blame_file(entry.path(), Some(&mut blame_options))?;
                for hunk in blame.iter() {
                    let person = people_db.find_by_signature(hunk.orig_signature())?;
                    owners.for_person(&person).add_hunk(&hunk);
                }
            }
        }
        progress.inc(1);
    }

    progress.finish();

    for (person, score) in owners.iter() {
        println!("{} has {} lines", person.name(), score.total_lines_owned);
    }

    Ok(())
}
