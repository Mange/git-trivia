use indicatif::{ProgressBar, ProgressStyle};
use git2::{Commit, BlameOptions, BlameHunk};

use super::errors::*;
use super::{TreeWalker, Context};
use person::CombinedTracking;

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

    println!("\n-- People --");
    for (person, score) in owners.people_iter() {
        println!("{} has {} lines", person.name(), score.total_lines_owned);
    }

    println!("\n-- Teams --");
    for (team_name, score) in owners.team_iter() {
        match team_name {
            Some(name) => println!("{} has {} lines", name, score.total_lines_owned),
            None => {
                println!(
                    "{} lines is owned by no team in particular",
                    score.total_lines_owned
                )
            }
        }
    }

    Ok(())
}
