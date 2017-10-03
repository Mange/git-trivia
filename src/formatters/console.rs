use person::CombinedTracking;
use ownership::OwnershipScore;
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

impl<'a, 'b> Format for &'a CombinedTracking<'b, OwnershipScore> {
    fn format(&self) -> Result<()> {
        println!("\n-- People --");
        for (person, score) in self.people_iter() {
            println!("{} has {} lines", person.name(), score.total_lines_owned);
        }

        println!("\n-- Teams --");
        for (team_name, score) in self.team_iter() {
            match team_name {
                Some(name) => println!("{} has {} lines\n", name, score.total_lines_owned),
                None => {
                    println!(
                        "{} lines is owned by no team in particular\n",
                        score.total_lines_owned
                    )
                }
            }
        }

        Ok(())
    }
}
