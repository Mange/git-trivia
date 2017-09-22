#[macro_use]
extern crate clap;
use clap::{AppSettings, SubCommand, Arg, ArgMatches};

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

    match matches.subcommand() {
        ("init", Some(args)) => init(args),
        // This should not happen considering SubcommandRequiredElseHelp setting above
        // It would happen if a new subcommand was added but not matched on here.
        _ => std::process::exit(1),
    };
}

fn init(args: &ArgMatches) {
    if args.is_present("dry_run") {
        match initialize_config() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Not yet implemented... :(");
        std::process::exit(1);
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
