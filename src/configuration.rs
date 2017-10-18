use std::collections::{HashMap, HashSet};

use git2::Signature;

use person::*;
use super::errors::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub generated_at_sha: String,
    pub people: Vec<Person>,
}

impl Configuration {
    pub fn people_db(&self) -> Result<PeopleDatabase> {
        let mut db = PeopleDatabase::new();
        for person in &self.people {
            db.add_person((*person).clone())?;
        }
        Ok(db)
    }
}

#[derive(Default, Debug)]
pub struct ConfigurationBuilder {
    generated_at_sha: Option<String>,

    seen_emails: HashSet<String>,
    people_by_name: HashMap<String, Person>,
}

impl ConfigurationBuilder {
    pub fn new() -> ConfigurationBuilder {
        ConfigurationBuilder::default()
    }

    pub fn from_existing(config: Configuration) -> ConfigurationBuilder {
        let mut builder = ConfigurationBuilder::default();
        builder.read_existing(config);
        builder
    }

    pub fn set_latest_commit_sha(&mut self, commit_sha: String) {
        self.generated_at_sha = Some(commit_sha);
    }

    pub fn add_author<'a>(&mut self, author: Signature<'a>) {
        if let Some(email) = author.email() {
            if !self.seen_emails.contains(email) {
                if let Some(name) = author.name() {
                    self.seen_emails.insert(email.into());
                    self.people_by_name
                        .entry(name.to_owned())
                        .or_insert_with(|| Person::new(name))
                        .add_email(email);
                }
            }
        }
    }

    pub fn into_configuration(mut self) -> Result<Configuration> {
        if self.generated_at_sha.is_none() {
            bail!("Repository has no commit yet");
        }

        let mut people: Vec<Person> = self.people_by_name.drain().map(|(_, v)| v).collect();
        people.sort();

        Ok(Configuration {
            generated_at_sha: self.generated_at_sha.unwrap(),
            people: people,
        })
    }

    fn read_existing(&mut self, config: Configuration) {
        self.generated_at_sha = Some(config.generated_at_sha);

        for person in config.people {
            let name = String::from(person.name());

            for email in person.emails() {
                self.seen_emails.insert(String::from(email));
            }

            self.people_by_name.insert(name, person);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_signature(name: &str, email: &str) -> Signature<'static> {
        Signature::now(name, email).unwrap()
    }

    fn email(email: &str) -> Email {
        Email::from(email)
    }

    #[test]
    fn it_builds_config_of_people() {
        let mut builder = ConfigurationBuilder::new();

        builder.set_latest_commit_sha(String::from("deadbeef"));
        builder.add_author(git_signature("Jane Doe", "jane.doe@example.com"));
        builder.add_author(git_signature("John Doe", "john.doe@example.com"));
        builder.add_author(git_signature("Jane Doe", "janed@example.com"));

        let config = builder.into_configuration().unwrap();
        let people_db = config.people_db().unwrap();

        assert_eq!(people_db.len(), 2);

        let jane = people_db
            .find_by_email(&email("jane.doe@example.com"))
            .unwrap();
        assert_eq!(jane.name(), "Jane Doe");
        assert_eq!(jane.emails().len(), 2);
        assert!(jane.emails().contains(&email("jane.doe@example.com")));
        assert!(jane.emails().contains(&email("janed@example.com")));

        let john = people_db
            .find_by_email(&email("john.doe@example.com"))
            .unwrap();
        assert_eq!(john.name(), "John Doe");
        assert_eq!(john.emails().len(), 1);
        assert!(john.emails().contains(&email("john.doe@example.com")));
    }

    #[test]
    fn it_sorts_people() {
        let mut builder = ConfigurationBuilder::new();

        builder.set_latest_commit_sha(String::from("deadbeef"));
        builder.add_author(git_signature("YYY", "yyy@example.com"));
        builder.add_author(git_signature("ZZZ", "zzz@example.com"));
        builder.add_author(git_signature("XXX", "xxx@example.com"));

        let config = builder.into_configuration().unwrap();

        let names: Vec<&str> = config.people.iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["XXX", "YYY", "ZZZ"]);
    }
}
