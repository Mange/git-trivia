#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate git2;

mod person;
use person::*;

mod configuration;
use configuration::Configuration;

use std::collections::HashMap;

fn main() {
    // Just a technology sample so far
    match initialize_config() {
        Ok(_) => {}
        Err(err) => {
            println!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn initialize_config() -> Result<(), String> {
    let repo = git2::Repository::open_from_env().map_err(
        |_| "Could not open repo",
    )?;
    let mut walker = repo.revwalk().unwrap();
    walker.push_head().expect("Could not push HEAD");

    let mut people_by_name = HashMap::new();

    for oid in walker.flat_map(Result::ok) {
        // The Oid comes from the Revwalker that only yields proper commit Oids. Unwrapping should
        // be safe.
        let commit = repo.find_commit(oid).unwrap();

        let author = commit.author();

        if let (Some(author_name), Some(author_email)) = (author.name(), author.email()) {
            people_by_name
                .entry(author_name.to_owned())
                .or_insert_with(|| Person::new(author_name))
                .add_email(author_email);
        }
    }

    let configuration =
        Configuration { people: people_by_name.into_iter().map(|(_, v)| v).collect() };

    println!(
        "{}",
        serde_yaml::to_string(&configuration).map_err(|err| {
            format!("Could not serialize to YAML: {}", err)
        })?
    );

    let people = configuration.people_db();
    println!("{:?}", people);

    Ok(())
}
