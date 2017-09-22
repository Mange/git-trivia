use super::errors::*;
use super::{TreeWalker, Context};

use git2::Commit;

pub fn calculate(context: &Context, commit: &Commit) -> Result<()> {
    let _people_db = context.people_db();
    let repo = context.repo();

    for entry in TreeWalker::new(repo, commit.tree()?) {
        println!("{}", entry.path().display());
    }

    bail!(ErrorKind::NotYetImplemented("Calculating ownership"));
}
