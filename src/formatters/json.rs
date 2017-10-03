extern crate serde;
extern crate serde_json;

use self::serde::ser::{Serialize, Serializer, SerializeStruct, SerializeMap};

use person::{CombinedTracking, PeopleTracking, TeamTracking};
use errors::*;

// The JSON formatter prints JSON to STDOUT
pub struct Formatter {}

pub trait Format {
    fn format(&self) -> Result<()>;
}

impl Formatter {
    pub fn display<F>(data: F) -> Result<()>
    where
        F: Format,
    {
        data.format().and_then(|_| {
            println!("");
            Ok(())
        })
    }
}

impl<T> Format for T
where
    T: Serialize,
{
    fn format(&self) -> Result<()> {
        serde_json::to_writer_pretty(::std::io::stdout(), self).map_err(|e| e.into())
    }
}

impl<'b, T> Serialize for CombinedTracking<'b, T>
where
    T: Default + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("CombinedTracking", 2)?;
        s.serialize_field("people", &self.people_tracking())?;
        s.serialize_field("teams", &self.team_tracking())?;
        s.end()
    }
}

impl<'b, T> Serialize for PeopleTracking<'b, T>
where
    T: Default + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.total_people()))?;
        for (person, value) in self.iter() {
            s.serialize_entry(person.name(), value)?;
        }
        s.end()
    }
}

impl<'b, T> Serialize for TeamTracking<'b, T>
where
    T: Default + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries = self.total_teams() + 1;
        let mut s = serializer.serialize_map(Some(entries))?;

        for (team_name, value) in self.iter() {
            s.serialize_entry(team_name.unwrap_or("(No team)"), value)?;
        }
        s.end()
    }
}
