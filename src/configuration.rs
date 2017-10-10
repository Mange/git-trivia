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
