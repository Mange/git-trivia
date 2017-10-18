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

use git2::{Repository, Oid};

mod formatters;

mod configuration;
pub use configuration::{Configuration, ConfigurationBuilder};

mod context;
use context::{Context, config_file_path};

mod tree_walker;
pub use tree_walker::TreeWalker;

mod person;
use person::*;

mod ownership;

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
                .about("Initializes a new config for repository.")
                .arg(Arg::with_name("dry_run").short("n").long("dry-run").visible_alias("stdout").help(
                    "Don't write generated config file to disk; instead output it on STDOUT.",
                ))
                .arg(Arg::with_name("force").short("f").long("force").help(
                    "Overwrite any existing trivia config file.",
                )),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Update config for repository")
                .arg(Arg::with_name("dry_run").short("n").long("dry-run").visible_alias("stdout").help(
                    "Don't write generated config file to disk; instead output it on STDOUT.",
                )),
        )
        .subcommand(
            SubCommand::with_name("ownership")
                .about("Calculates line ownership")
        );
    let matches = app.get_matches();

    match matches.subcommand() {
        ("init", Some(args)) => init(args),
        ("update", Some(args)) => update(args),
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

fn update(args: &ArgMatches) -> Result<()> {
    let repo = Repository::open_from_env()?;
    let config = context::load_configuration(&repo)?;
    if config.generated_at_sha == current_head_sha(&repo)? {
        eprintln!("Config already up to date.");
        Ok(())
    } else {
        let config_file_path = config_file_path(&repo);
        let new_config_yaml_string = update_config(&repo, config).chain_err(
            || "Could not update config",
        )?;
        if args.is_present("dry_run") {
            eprintln!(
                "Would write to this file: {}",
                config_file_path.to_string_lossy()
            );
            println!("{}", new_config_yaml_string);
            Ok(())
        } else {
            let mut file = File::create(&config_file_path)?;
            file.write_all(new_config_yaml_string.as_bytes())?;
            file.write_all(b"\n")?; // Write a trailing newline; that looks so much better
            eprintln!("Configuration updated in {}", config_file_path.display());
            Ok(())
        }
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
    let mut config_builder = ConfigurationBuilder::new();
    let mut walker = repo.revwalk().unwrap();

    config_builder.set_latest_commit_sha(current_head_sha(repo)?);

    walker.push_head()?;
    let commits = walker.flat_map(std::result::Result::ok).flat_map(|oid| {
        repo.find_commit(oid)
    });

    for commit in commits {
        config_builder.add_author(commit.author());
    }

    let configuration = config_builder.into_configuration()?;

    Ok(serde_yaml::to_string(&configuration)?)
}

fn update_config(repo: &Repository, configuration: Configuration) -> Result<String> {
    let old_head = configuration.generated_at_sha.clone();

    let mut config_builder = ConfigurationBuilder::from_existing(configuration);
    let mut walker = repo.revwalk().unwrap();

    config_builder.set_latest_commit_sha(current_head_sha(repo)?);

    walker.push_head()?;
    let old_head_oid = Oid::from_str(&old_head).chain_err(
        || "Could not parse generated_at_sha configuration SHA",
    )?;

    walker.hide(old_head_oid)?;

    let commits = walker.flat_map(std::result::Result::ok).flat_map(|oid| {
        repo.find_commit(oid)
    });

    for commit in commits {
        config_builder.add_author(commit.author());
    }

    let configuration = config_builder.into_configuration()?;

    Ok(serde_yaml::to_string(&configuration)?)
}

fn current_head_sha(repo: &Repository) -> Result<String> {
    Ok(repo.head()?.resolve()?.target().unwrap().to_string())
}
