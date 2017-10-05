extern crate term;

use std::io::prelude::*;

use prettytable::Table;
use term::{Attr, color};

use ownership::OwnershipStatistics;
use errors::*;

fn new_table() -> Table {
    use prettytable::format;

    let mut table = Table::new();
    table.set_format(
        format::FormatBuilder::new()
            .column_separator('│')
            .borders('│')
            .separators(
                &[format::LinePosition::Top],
                format::LineSeparator::new('─', '┬', '┌', '┐'),
            )
            .separators(
                &[format::LinePosition::Intern],
                format::LineSeparator::new('─', '┼', '├', '┤'),
            )
            .separators(
                &[format::LinePosition::Bottom],
                format::LineSeparator::new('─', '┴', '└', '┘'),
            )
            .padding(1, 1)
            .build(),
    );
    table
}

// Console formatter will just print to STDOUT, so no need to even return anything.
pub struct Formatter {}

pub struct Terminal {
    term: Option<Box<term::StdoutTerminal>>,
}

impl Write for Terminal {
    fn write(&mut self, d: &[u8]) -> ::std::result::Result<usize, ::std::io::Error> {
        match self.term {
            Some(ref mut term) => term.write(d),
            None => ::std::io::stdout().write(d),
        }
    }

    fn flush(&mut self) -> ::std::result::Result<(), ::std::io::Error> {
        match self.term {
            Some(ref mut term) => term.flush(),
            None => ::std::io::stdout().flush(),
        }
    }
}

impl Terminal {
    fn stdout() -> Terminal {
        Terminal { term: term::stdout() }
    }

    fn attr(&mut self, attr: Attr) -> Result<()> {
        match self.term {
            Some(ref mut term) => {
                match term.attr(attr) {
                    Ok(_) |
                    Err(term::Error::NotSupported) |
                    Err(term::Error::ColorOutOfRange) => Ok(()),
                    Err(error) => Err(error.into()),
                }
            }
            None => {
                // Do nothing
                Ok(())
            }
        }
    }

    fn reset(&mut self) -> Result<()> {
        match self.term {
            Some(ref mut term) => {
                match term.reset() {
                    Ok(_) |
                    Err(term::Error::NotSupported) |
                    Err(term::Error::ColorOutOfRange) => Ok(()),
                    Err(error) => Err(error.into()),
                }
            }
            None => {
                // Do nothing
                Ok(())
            }
        }
    }

    fn is_term(&self) -> bool {
        self.term.is_some()
    }

    fn terminal_width(&self) -> Option<usize> {
        use terminal_size::{Width, terminal_size};

        match terminal_size() {
            Some((Width(w), _)) => Some(w as usize),
            _ => None,
        }
    }

    fn print_fact<S>(&mut self, title: &str, value: S) -> Result<()>
    where
        S: ::std::fmt::Display,
    {
        self.attr(Attr::Bold)?;
        write!(self, "{}: ", title)?;
        self.reset()?;
        writeln!(self, "{}", value)?;
        Ok(())
    }

    fn print_hr(&mut self, width: usize) -> Result<()> {
        writeln!(self, "{}", "─".repeat(width)).map_err(|e| e.into())
    }

    fn print_header(&mut self, title: &str) -> Result<()> {
        self.attr(Attr::ForegroundColor(color::CYAN))?;
        let width = self.terminal_width().unwrap_or(title.len() as usize);
        self.print_hr(width)?;
        writeln!(self, "{title:^width$}", title = title, width = width)?;
        self.print_hr(width)?;
        self.reset()?;
        Ok(())
    }

    fn print_headline(&mut self, title: &str) -> Result<()> {
        self.attr(Attr::ForegroundColor(color::BRIGHT_CYAN))?;
        if self.is_term() {
            writeln!(self, "{}", title)?;
        } else {
            writeln!(self, "-- {} --", title)?;
        }
        self.reset()?;
        Ok(())
    }
}

pub trait Format {
    fn format(&self, &mut Terminal) -> Result<()>;
}

impl Formatter {
    pub fn display<F>(data: F) -> Result<()>
    where
        F: Format,
    {
        let mut terminal = Terminal::stdout();
        data.format(&mut terminal)
    }
}

impl<'a, 'b> Format for &'a OwnershipStatistics<'b> {
    fn format(&self, terminal: &mut Terminal) -> Result<()> {
        terminal.print_header("Ownership details")?;

        terminal.print_fact("Total lines", self.total_lines())?;

        terminal.print_headline("\nPeople")?;
        let mut people_table = new_table();
        people_table.add_row(
            row![b->"#", b->"Person", b->"Lines owned", b->"Percent of total"],
        );

        for (index, &(person, ref score)) in self.people_toplist().iter().enumerate() {
            let place = (index + 1).to_string();
            let name = person.name();
            let lines = score.total_lines_owned.to_string();
            let percent = format!("{:6.2}%", score.percent_owned());

            people_table.add_row(row![place, name, lines, percent]);
        }
        people_table.printstd();

        terminal.print_headline("\nTeams")?;
        let mut teams_table = new_table();
        teams_table.add_row(
            row![b->"#", b->"Person", b->"Lines owned", b->"Percent of total"],
        );

        for (index, &(ref team_name, ref score)) in self.teams_toplist().iter().enumerate() {
            let place = (index + 1).to_string();
            let name = match *team_name {
                Some(name) => name,
                None => "(Others)",
            };
            let lines = score.total_lines_owned.to_string();
            let percent = format!("{:6.2}%", score.percent_owned());

            teams_table.add_row(row![place, name, lines, percent]);
        }
        teams_table.printstd();

        Ok(())
    }
}
