extern crate git2;

mod person;
use person::*;

fn main() {
    // Just a technology sample so far
    match list_people() {
        Ok(_) => {}
        Err(err) => {
            println!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn list_people() -> Result<(), String> {
    let repo = git2::Repository::open_from_env().map_err(
        |_| "Could not open repo",
    )?;
    let mut walker = repo.revwalk().unwrap();
    walker.push_head().expect("Could not push HEAD");

    let mut people = PeopleDatabase::new();

    for oid in walker.flat_map(Result::ok) {
        // The Oid comes from the Revwalker that only yields proper commit Oids. Unwrapping should
        // be safe.
        let commit = repo.find_commit(oid).unwrap();

        let author = commit.author();
        if let Some(author_email) = author.email() {
            let email = Email::from(author_email);
            if !people.has_email(&email) {
                let mut person = Person::new(author.name().unwrap_or("(No name)"));
                person.add_email(email);
                people.add_person(person).map_err(|err| {
                    match err {
                        PeopleDatabaseError::ConflictingEmail { new, existing, email } => {
                            format!(
                                "Could not add {new_name} to people database due to conflicting email with {existing_name}. Email that both has: {email}",
                                new_name = new.name(),
                                existing_name = existing.name(),
                                email = email
                            )
                        }
                    }
                })?;
            }
        }
    }

    println!("{:?}", people);
    Ok(())
}
