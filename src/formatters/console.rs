use ownership::OwnershipStatistics;
use errors::*;

// Console formatter will just print to STDOUT, so no need to even return anything.
pub struct Formatter {}

pub trait Format {
    fn format(&self) -> Result<()>;
}

impl Formatter {
    pub fn display<F>(data: F) -> Result<()>
    where
        F: Format,
    {
        data.format()
    }
}

impl<'a, 'b> Format for &'a OwnershipStatistics<'b> {
    fn format(&self) -> Result<()> {
        println!("\n--- Ownership details ---");
        println!("Total lines: {}", self.total_lines());

        println!("\n-- People --");
        for (person, score) in self.people_toplist() {
            println!(
                "{} has {} lines ({:6.2}% of all lines)",
                person.name(),
                score.total_lines_owned,
                score.percent_owned()
            );
        }

        println!("\n-- Teams --");
        for (team_name, score) in self.teams_toplist() {
            match team_name {
                Some(name) => {
                    println!(
                        "{} has {} lines ({:6.2}% of all lines)",
                        name,
                        score.total_lines_owned,
                        score.percent_owned()
                    )
                }
                None => {
                    println!(
                        "{} lines is owned by no team in particular ({:6.2}% of all lines)",
                        score.total_lines_owned,
                        score.percent_owned()
                    )
                }
            }
        }

        Ok(())
    }
}
