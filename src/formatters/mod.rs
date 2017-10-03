mod json;
mod console;

use clap::ArgMatches;
use super::errors::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Format {
    Console,
    JSON,
}

impl Format {
    pub fn display<F>(&self, data: F) -> Result<()>
    where
        F: json::Format + console::Format,
    {
        match *self {
            Format::Console => console::Formatter::display(data),
            Format::JSON => json::Formatter::display(data),
        }
    }
}

pub static POSSIBLE_VALUES: &'static [&'static str] = &["console", "json"];

pub fn from_args(args: &ArgMatches) -> Result<Format> {
    match args.value_of("format") {
        Some("console") | None => Ok(Format::Console),
        Some("json") => Ok(Format::JSON),
        Some(other) => bail!("Not a valid format: {}", other),
    }
}
