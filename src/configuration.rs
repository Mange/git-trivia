use person::*;

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub people: Vec<Person>,
}

impl Configuration {
    pub fn people_db(&self) -> PeopleDatabase {
        let mut db = PeopleDatabase::new();
        for person in self.people.iter() {
            db.add_person((*person).clone());
        }
        db
    }
}
