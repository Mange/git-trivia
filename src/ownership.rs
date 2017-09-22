use super::errors::*;
use super::Context;

use git2::Commit;

pub fn calculate(context: &Context, commit: &Commit) -> Result<()> {
    let _people_db = context.people_db();
    let _repo = context.repo();

    for entry in &commit.tree()? {
        println!("{}", entry.name().unwrap_or("No name"));
    }

    bail!(ErrorKind::NotYetImplemented("Calculating ownership"));
}
