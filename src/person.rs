use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::cmp::{PartialEq, Eq, Ord, Ordering};

use git2::Signature;

use super::errors::*;

#[derive(Debug, PartialEq, Hash, Clone, Deserialize, Serialize)]
pub struct Email(String);

impl<'a> From<&'a str> for Email {
    fn from(string: &'a str) -> Email {
        Email(string.to_owned())
    }
}

impl From<String> for Email {
    fn from(string: String) -> Email {
        Email(string)
    }
}

impl<'a> From<&'a String> for Email {
    fn from(string: &'a String) -> Email {
        Email(string.clone())
    }
}

impl Eq for Email {}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Person {
    name: String,
    emails: HashSet<Email>,
    #[serde(rename = "team")]
    team_name: Option<String>,
}

impl PartialEq<Email> for Person {
    fn eq(&self, other: &Email) -> bool {
        self.emails.contains(other)
    }
}

impl Hash for Person {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Person {
    fn eq(&self, rhs: &Person) -> bool {
        self.name == rhs.name
    }
}

impl Eq for Person {}

impl PartialOrd for Person {
    fn partial_cmp(&self, other: &Person) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for Person {
    fn cmp(&self, other: &Person) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Person {
    pub fn new<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Person {
            name: name.into(),
            emails: HashSet::new(),
            team_name: None,
        }
    }

    pub fn set_team_name<S>(&mut self, name: S)
    where
        S: Into<Option<String>>,
    {
        self.team_name = name.into();
    }

    pub fn add_email<E>(&mut self, email: E) -> bool
    where
        E: Into<Email>,
    {
        self.emails.insert(email.into())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn team_name(&self) -> Option<&str> {
        self.team_name.as_ref().map(String::as_ref)
    }

    pub fn emails(&self) -> &HashSet<Email> {
        &self.emails
    }

    pub fn has_email(&self, email: &Email) -> bool {
        self.emails.contains(email)
    }
}

#[derive(Debug, Default)]
pub struct PeopleDatabase {
    people: Vec<Person>,
    lookup: HashMap<Email, usize>,
}

impl PeopleDatabase {
    pub fn new() -> PeopleDatabase {
        PeopleDatabase::default()
    }

    pub fn add_person(&mut self, person: Person) -> Result<()> {
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
                let existing = self.find_by_email(email).unwrap();
                bail!(ErrorKind::ConflictingEmail(
                    existing.name().to_string(),
                    person.name().to_string(),
                    email.clone(),
                ));
            }
        }
    }

    pub fn has_email(&self, email: &Email) -> bool {
        self.lookup.contains_key(email)
    }

    pub fn find_by_email(&self, email: &Email) -> Result<&Person> {
        self.lookup
            .get(email)
            .and_then(|index| self.people.get(*index))
            .ok_or_else(|| ErrorKind::UnknownEmail(email.to_owned()).into())
    }

    pub fn find_by_signature(&self, signature: Signature) -> Result<&Person> {
        let email = signature.email().unwrap_or("");
        self.find_by_email(&email.into())
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

#[derive(Debug, Default)]
pub struct PeopleTracking<'people, T>
where
    T: Default,
{
    lookup: HashMap<&'people Person, T>,
}

impl<'people, T> PeopleTracking<'people, T>
where
    T: Default,
{
    pub fn new() -> Self {
        PeopleTracking::default()
    }

    pub fn for_person(&mut self, person: &'people Person) -> &mut T {
        self.lookup.entry(person).or_insert_with(Default::default)
    }

    pub fn person_value(&self, person: &Person) -> Option<&T> {
        self.lookup.get(person)
    }

    pub fn iter(&self) -> ::std::collections::hash_map::Iter<&Person, T> {
        self.lookup.iter()
    }
}

#[derive(Debug, Default)]
pub struct TeamTracking<'people, T>
where
    T: Default,
{
    no_team: T,
    lookup: HashMap<&'people str /* team_name */, T>,
}

impl<'people, T> TeamTracking<'people, T>
where
    T: Default,
{
    pub fn new() -> Self {
        TeamTracking::default()
    }

    pub fn for_person(&mut self, person: &'people Person) -> &mut T {
        match person.team_name() {
            Some(name) => self.for_team_name(name),
            None => self.for_no_team(),
        }
    }

    pub fn for_team_name(&mut self, team_name: &'people str) -> &mut T {
        self.lookup.entry(team_name).or_insert_with(
            Default::default,
        )
    }

    pub fn for_no_team(&mut self) -> &mut T {
        &mut self.no_team
    }

    pub fn team_value(&self, team_name: &str) -> Option<&T> {
        self.lookup.get(team_name)
    }

    pub fn no_team_value(&self) -> &T {
        &self.no_team
    }

    pub fn iter(&self) -> TeamTrackingIter<T> {
        TeamTrackingIter {
            emitted_no_team: false,
            no_team_value: self.no_team_value(),
            inner: self.lookup.iter(),
        }
    }
}

pub struct TeamTrackingIter<'a, T: 'a> {
    emitted_no_team: bool,
    no_team_value: &'a T,
    inner: ::std::collections::hash_map::Iter<'a, &'a str, T>,
}

impl<'a, T> Iterator for TeamTrackingIter<'a, T> {
    type Item = (Option<&'a str>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.emitted_no_team {
            self.emitted_no_team = true;
            Some((None, self.no_team_value))
        } else {
            match self.inner.next() {
                Some((name, value)) => Some((Some(name), value)),
                None => None,
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct CombinedTracking<'people, T>
where
    T: Default,
{
    people_tracking: PeopleTracking<'people, T>,
    team_tracking: TeamTracking<'people, T>,
}

impl<'people, T> CombinedTracking<'people, T>
where
    T: Default,
{
    pub fn new() -> Self {
        CombinedTracking::default()
    }

    pub fn track_person<F>(&mut self, person: &'people Person, mut func: F)
    where
        F: FnMut(&mut T),
    {
        func(self.people_tracking.for_person(person));
        func(self.team_tracking.for_person(person));
    }

    pub fn person_value(&self, person: &Person) -> Option<&T> {
        self.people_tracking.person_value(person)
    }

    pub fn team_value(&self, team_name: &str) -> Option<&T> {
        self.team_tracking.team_value(team_name)
    }

    pub fn no_team_value(&self) -> &T {
        self.team_tracking.no_team_value()
    }

    pub fn people_iter(&self) -> ::std::collections::hash_map::Iter<&Person, T> {
        self.people_tracking.iter()
    }

    pub fn team_iter(&self) -> TeamTrackingIter<T> {
        self.team_tracking.iter()
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
    fn it_does_not_add_duplicted_emails() {
        let mut person = Person::new("Jane Doe");
        assert_eq!(person.add_email("jane@example.com"), true);
        assert_eq!(person.add_email("jane@example.com"), false);
        assert_eq!(person.add_email("jane2@example.com"), true);
        assert_eq!(person.emails().len(), 2);
    }

    #[test]
    fn it_finds_by_email_in_people_database() {
        let mut joe = Person::new("John Doe");
        let mut jane = Person::new("Jane Doe");

        joe.add_email("john@example.com");

        jane.add_email("jane@example.com");
        jane.add_email("doe@example.com");

        let mut db = PeopleDatabase::new();
        assert!(db.add_person(joe).is_ok());
        assert!(db.add_person(jane).is_ok());

        assert_eq!(
            db.find_by_email(&Email::from("john@example.com"))
                .map(|p| p.name())
                .unwrap(),
            "John Doe"
        );

        assert_eq!(
            db.find_by_email(&Email::from("jane@example.com"))
                .map(|p| p.name())
                .unwrap(),
            "Jane Doe"
        );

        assert_eq!(
            db.find_by_email(&Email::from("doe@example.com"))
                .map(|p| p.name())
                .unwrap(),
            "Jane Doe"
        );

        assert_eq!(
            db.find_by_email(&Email::from("unknown@example.com"))
                .unwrap_err()
                .to_string(),
            "Unknown email: \"unknown@example.com\"\nPlease add it to a person in the configuration file."
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

        assert_eq!(
            err.to_string(),
            "Multiple people with the same email: doe@example.com is used by John Doe and Jane Doe.\nPlease put this email under only a single person."
        );
    }

    #[test]
    fn it_tracks_people() {
        #[derive(PartialEq, Eq, Debug, Default)]
        struct Stub {
            counter: i32,
        };

        impl Stub {
            fn incr(&mut self) {
                self.counter += 1;
            }

            fn current(&self) -> i32 {
                self.counter
            }
        }

        let joe = Person::new("John Doe");
        let jane = Person::new("Jane Doe");

        let mut people_tracking: PeopleTracking<Stub> = PeopleTracking::new();

        people_tracking.for_person(&joe).incr();
        assert_eq!(people_tracking.for_person(&joe).current(), 1);
        assert_eq!(people_tracking.for_person(&jane).current(), 0);
    }

    #[test]
    fn it_tracks_teams() {
        #[derive(PartialEq, Eq, Debug, Default)]
        struct Stub {
            counter: i32,
        };

        impl Stub {
            fn incr(&mut self) {
                self.counter += 1;
            }

            fn current(&self) -> i32 {
                self.counter
            }
        }

        let mut joe = Person::new("John Doe");
        joe.set_team_name(String::from("Team 1"));
        let joe = joe;

        let mut jane = Person::new("Jane Doe");
        jane.set_team_name(None);
        let jane = jane;

        let mut team_tracking: TeamTracking<Stub> = TeamTracking::new();

        team_tracking.for_person(&joe).incr();
        team_tracking.for_person(&jane).incr();

        assert_eq!(team_tracking.for_person(&joe).current(), 1);
        assert_eq!(team_tracking.for_person(&jane).current(), 1);
        assert_eq!(team_tracking.no_team_value().current(), 1);
    }

    #[test]
    fn it_tracks_combined_teams_and_people() {
        #[derive(PartialEq, Eq, Debug, Default)]
        struct Stub {
            counter: i32,
        };

        impl Stub {
            fn incr(&mut self) {
                self.counter += 1;
            }
        }

        let mut joe = Person::new("John Doe");
        joe.set_team_name(String::from("Team 1"));
        let joe = joe;

        let mut jane = Person::new("Jane Doe");
        jane.set_team_name(None);
        let jane = jane;

        let mut tracking: CombinedTracking<Stub> = CombinedTracking::new();

        tracking.track_person(&joe, |e| e.incr());
        tracking.track_person(&jane, |e| e.incr());
        tracking.track_person(&jane, |e| e.incr());

        assert_eq!(tracking.person_value(&joe), Some(&Stub { counter: 1 }));
        assert_eq!(tracking.person_value(&jane), Some(&Stub { counter: 2 }));
        assert_eq!(tracking.team_value("Team 1"), Some(&Stub { counter: 1 }));
        assert_eq!(tracking.no_team_value(), &Stub { counter: 2 });
    }
}
