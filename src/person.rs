use std::collections::HashMap;
use std::fmt;

#[derive(Debug, PartialEq, Hash, Clone)]
pub struct Email(String);

impl<'a> From<&'a str> for Email {
    fn from(string: &'a str) -> Email {
        Email(string.to_owned())
    }
}

impl Eq for Email {}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct Person {
    name: String,
    emails: Vec<Email>,
}

impl PartialEq<Email> for Person {
    fn eq(&self, other: &Email) -> bool {
        self.emails.contains(other)
    }
}

impl Person {
    pub fn new<S>(name: S) -> Person
    where
        S: Into<String>,
    {
        Person {
            name: name.into(),
            emails: vec![],
        }
    }

    pub fn add_email<E>(&mut self, email: E) -> ()
    where
        E: Into<Email>,
    {
        self.emails.push(email.into())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn emails(&self) -> &Vec<Email> {
        &self.emails
    }
}

#[derive(Debug, Default)]
pub struct PeopleDatabase {
    people: Vec<Person>,
    lookup: HashMap<Email, usize>,
}

#[derive(Debug)]
pub enum PeopleDatabaseError<'db> {
    ConflictingEmail {
        new: Person,
        existing: &'db Person,
        email: Email,
    },
}

impl PeopleDatabase {
    pub fn new() -> PeopleDatabase {
        PeopleDatabase::default()
    }

    pub fn add_person<'db>(&'db mut self, person: Person) -> Result<(), PeopleDatabaseError<'db>> {
        // This whole method turns out the be very ugly due to Rusts borrowchecker not being too
        // clever yet. (Non-lexical lifetimes, etc.)
        //
        // Basically, we cannot use any methods borrowing &self before the lines that uses
        // &mut self.

        // Check for conflict first
        let emails_copy = person.emails.clone(); // Clone to appease borrowchk
        let conflict_email = emails_copy.iter().find(|email| self.has_email(email));

        match conflict_email {
            None => {
                self.insert_person(person);
                Ok(())
            }
            Some(email) => {
                Err(PeopleDatabaseError::ConflictingEmail {
                    existing: self.find_by_email(email).unwrap(),
                    new: person,
                    email: email.clone(),
                })
            }
        }

    }

    pub fn has_email(&self, email: &Email) -> bool {
        self.lookup.contains_key(email)
    }

    pub fn find_by_email(&self, email: &Email) -> Option<&Person> {
        match self.lookup.get(email) {
            Some(index) => self.people.get(*index),
            None => None,
        }
    }

    fn insert_person(&mut self, person: Person) {
        // No conflicts, add to lookup table
        let index = self.people.len();
        for email in person.emails().iter() {
            self.lookup.insert(email.clone(), index);
        }
        self.people.push(person);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_equals_string_equal_to_one_of_the_emails() {
        let mut person = Person::new("John Doe");
        person.add_email(Email::from("j.doe@example.com"));
        person.add_email("doe@example.com");
        let person = person;

        assert_eq!(person, Email::from("j.doe@example.com"));
        assert_eq!(person, Email::from("doe@example.com"));

        assert_ne!(person, Email::from("John Doe"));
        assert_ne!(person, Email::from("doe.does@example.com"));
    }

    #[test]
    fn it_finds_by_email_in_people_database() {
        let mut joe = Person::new("John Doe");
        let mut jane = Person::new("Jane Doe");

        joe.add_email("john@example.com");

        jane.add_email("jane@example.com");
        jane.add_email("doe@example.com");

        let mut db = PeopleDatabase::new();
        db.add_person(joe);
        db.add_person(jane);

        assert_eq!(
            db.find_by_email(&Email::from("john@example.com")).map(
                |p| {
                    p.name()
                },
            ),
            Some("John Doe")
        );

        assert_eq!(
            db.find_by_email(&Email::from("jane@example.com")).map(
                |p| {
                    p.name()
                },
            ),
            Some("Jane Doe")
        );

        assert_eq!(
            db.find_by_email(&Email::from("doe@example.com")).map(|p| {
                p.name()
            }),
            Some("Jane Doe")
        );

        assert!(
            db.find_by_email(&Email::from("unknown@example.com"))
                .is_none()
        );
    }

    #[test]
    fn it_does_not_allow_conflicting_emails_in_people_database() {
        let mut joe = Person::new("John Doe");
        joe.add_email("doe@example.com");

        let mut jane = Person::new("Jane Doe");
        jane.add_email("doe@example.com");

        let mut db = PeopleDatabase::new();
        assert!(db.add_person(joe).is_ok());

        let err = db.add_person(jane).unwrap_err();

        match err {
            PeopleDatabaseError::ConflictingEmail {
                new,
                existing,
                email,
            } => {
                assert_eq!(new.name(), "Jane Doe");
                assert_eq!(existing.name(), "John Doe");
                assert_eq!(email, Email::from("doe@example.com"));
            }
            _ => panic!("Did not get a ConflictingEmail; got a {:?}", err),
        }
    }
}
