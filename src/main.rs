#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate clap;
use clap::{AppSettings, SubCommand, Arg, ArgMatches};

#[macro_use]
extern crate serde_derive;

extern crate serde_yaml;
extern crate git2;
use git2::Repository;

mod person;
use person::*;

mod configuration;
use configuration::Configuration;

use std::collections::HashMap;

mod errors {
    error_chain! {
        foreign_links {
            GitError(super::git2::Error);
            YamlError(super::serde_yaml::Error);
        }

        errors {
            NotYetImplemented(feature: &'static str) {
                description("Not yet implemented")
                display("{} is not yet implemented. This is still a tech demo.", feature)
            }
            ConflictingEmail(name_a: String, name_b: String, email: super::Email) {
                description("Multiple people with the same email")
                display(
                    "Multiple people with the same email: {email} is used by {a} and {b}.\nPlease put this email under only a single person.",
                    email = email,
                    a = name_a,
                    b = name_b
                )
            }
        }
    }
}

pub use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    let app = app_from_crate!()
        .about(
            "Calculates fun and useless trivia about your Git repository.",
        )
        .global_setting(AppSettings::ColoredHelp)
        .global_setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::InferSubcommands)
        .subcommand(
            SubCommand::with_name("init")
                .about("Initializes config for repository")
                .arg(Arg::with_name("dry_run").short("n").long("dry-run").visible_alias("stdout").help(
                    "Don't write generated config file to disk; instead output it on STDOUT.",
                )),
        );
    let matches = app.get_matches();

    let repo = Repository::open_from_env()?;

    match matches.subcommand() {
        ("init", Some(args)) => init(args, &repo),
        // This should not happen considering SubcommandRequiredElseHelp setting above
        // It would happen if a new subcommand was added but not matched on here.
        _ => std::process::exit(1),
    }
}

fn init(args: &ArgMatches, repo: &Repository) -> Result<()> {
    if args.is_present("dry_run") {
        initialize_config(repo)
    } else {
        bail!(ErrorKind::NotYetImplemented("Writing config file"))
    }
}

fn initialize_config(repo: &Repository) -> Result<()> {
    let mut walker = repo.revwalk().unwrap();
    walker.push_head().expect("Could not push HEAD");

    let mut people_by_name = HashMap::new();

    for oid in walker.flat_map(std::result::Result::ok) {
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

    println!("{}", serde_yaml::to_string(&configuration)?);

    let people = configuration.people_db();
    println!("{:?}", people);

    Ok(())
}
