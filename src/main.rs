#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate clap;
use clap::{AppSettings, SubCommand, Arg, ArgMatches};

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate prettytable;

extern crate git2;
extern crate indicatif;
extern crate serde_json;
extern crate serde_yaml;
extern crate term;
extern crate terminal_size;

use git2::Repository;

mod formatters;

mod configuration;
pub use configuration::Configuration;

mod context;
use context::{Context, config_file_path};

mod tree_walker;
pub use tree_walker::TreeWalker;

mod person;
use person::*;

mod ownership;

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;

mod errors {
    error_chain! {
        foreign_links {
            GitError(super::git2::Error);
			JsonError(super::serde_json::Error);
            YamlError(super::serde_yaml::Error);
            IoError(super::std::io::Error);
            TerminalError(super::term::Error);
        }

        errors {
            NotYetImplemented(feature: &'static str) {
                description("Not yet implemented")
                display("{} is not yet implemented. This is still a tech demo.", feature)
            }
            ConfigFileExists(path: ::std::path::PathBuf) {
                description("Config file already exists")
                display("Config file already exists: {}", path.display())
            }
            ConfigNotFound(path: ::std::path::PathBuf) {
                description("Config file not found")
                display("Config file not found in {}.\nHint: Maybe you need to run the \"init\" command first?", path.display())
            }
            UnknownEmail(email: super::Email) {
                description("Unknown email")
                display("Unknown email: \"{}\"\nPlease add it to a person in the configuration file.", email)
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
        .arg(
            Arg::with_name("format")
                .short("F")
                .long("format")
                .visible_alias("formatter")
                .takes_value(true)
                .global(true)
                .possible_values(formatters::POSSIBLE_VALUES)
                .default_value("console")
                .help(
                    "Set the output format of this action."
                )
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initializes config for repository")
                .arg(Arg::with_name("dry_run").short("n").long("dry-run").visible_alias("stdout").help(
                    "Don't write generated config file to disk; instead output it on STDOUT.",
                ))
                .arg(Arg::with_name("force").short("f").long("force").help(
                    "Overwrite any existing trivia config file.",
                )),
        )
        .subcommand(
            SubCommand::with_name("ownership")
                .about("Calculates line ownership")
        );
    let matches = app.get_matches();

    match matches.subcommand() {
        ("init", Some(args)) => init(args),
        ("ownership", Some(args)) => ownership(args),
        // This should not happen considering SubcommandRequiredElseHelp setting above
        // It would happen if a new subcommand was added but not matched on here.
        _ => std::process::exit(1),
    }
}

fn init(args: &ArgMatches) -> Result<()> {
    let repo = Repository::open_from_env()?;
    let config_yaml_string = generate_initial_config(&repo)?;
    let config_file_path = config_file_path(&repo);
    let file_exists = config_file_path.exists();

    let force = args.is_present("force");

    if args.is_present("dry_run") {
        if file_exists && !force {
            eprintln!(
                "WARNING: Would not write to config file as it already exists: {}",
                config_file_path.to_string_lossy()
            );
        } else {
            eprintln!(
                "Would write to this file: {}",
                config_file_path.to_string_lossy()
            );
        }
        println!("{}", config_yaml_string);
        Ok(())
    } else if file_exists && !force {
        bail!(ErrorKind::ConfigFileExists(config_file_path));
    } else {
        let mut file = File::create(&config_file_path)?;
        file.write_all(config_yaml_string.as_bytes())?;
        file.write_all(b"\n")?; // Write a trailing newline; that looks so much better
        eprintln!("Configuration created in {}", config_file_path.display());
        Ok(())
    }
}

fn ownership(args: &ArgMatches) -> Result<()> {
    let format = formatters::from_args(args)?;

    let context = Context::load()?;
    let head_commit = context.head_commit()?;

    let owners = ownership::calculate(&context, &head_commit)?;
    format.display(&owners)
}

fn generate_initial_config(repo: &Repository) -> Result<String> {
    let mut walker = repo.revwalk().unwrap();
    walker.push_head().expect("Could not push HEAD");

    let mut people_by_name = HashMap::new();
    let mut emails_without_names: HashSet<String> = HashSet::new();
    let mut seen_emails: HashSet<String> = HashSet::new();

    for oid in walker.flat_map(std::result::Result::ok) {
        // The Oid comes from the Revwalker that only yields proper commit Oids. Unwrapping should
        // be safe.
        let commit = repo.find_commit(oid).unwrap();

        let author = commit.author();

        if let Some(author_email) = author.email() {
            if !seen_emails.contains(author_email) {
                seen_emails.insert(author_email.into());
                if let Some(author_name) = author.name() {
                    people_by_name
                        .entry(author_name.to_owned())
                        .or_insert_with(|| Person::new(author_name))
                        .add_email(author_email);
                } else {
                    emails_without_names.insert(author_email.into());
                }
            }
        }
    }

    // Some of the emails might have gotten matches with names later. Filter those out.
    emails_without_names.retain(|email| {
        let email: Email = email.into();
        !people_by_name.iter().any(
            |(_, person)| person.has_email(&email),
        )
    });

    // The ones that are left will get a name equal to their email address
    for email in &emails_without_names {
        people_by_name
            .entry(email.to_owned())
            .or_insert_with(|| Person::new(email.to_owned()))
            .add_email(email);
    }

    let mut people_list: Vec<Person> = people_by_name.into_iter().map(|(_, v)| v).collect();
    people_list.sort();

    let head_sha = repo.head()?.resolve()?.target().unwrap().to_string();

    let configuration = Configuration {
        people: people_list,
        generated_at_sha: head_sha,
    };

    Ok(serde_yaml::to_string(&configuration)?)
}
