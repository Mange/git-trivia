extern crate serde_yaml;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use git2::{Commit, Repository};

use super::Configuration;
use person::PeopleDatabase;
use super::errors::*;

pub struct Context {
    repository: Repository,
    config: Configuration,
    people_db: PeopleDatabase,
}

pub fn config_file_path(repo: &Repository) -> PathBuf {
    repo.path().join("trivia.yml")
}

impl Context {
    pub fn load() -> Result<Context> {
        let repo = Repository::open_from_env()?;
        let config = load_configuration(&repo)?;
        let people_db = config.people_db();

        Ok(Context {
            repository: repo,
            config: config,
            people_db: people_db,
        })
    }

    pub fn people_db(&self) -> &PeopleDatabase {
        &self.people_db
    }

    pub fn repo(&self) -> &Repository {
        &self.repository
    }

    pub fn head_commit(&self) -> Result<Commit> {
        let head_reference = self.repository.head().and_then(|r| r.resolve())?;

        match head_reference.target() {
            Some(oid) => Ok(self.repository.find_commit(oid)?),
            None => bail!("HEAD does not point to a valid SHA"),
        }
    }
}

fn load_configuration(repo: &Repository) -> Result<Configuration> {
    let path = config_file_path(repo);
    if path.exists() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let configuration: Configuration = serde_yaml::from_reader(reader)?;
        Ok(configuration)
    } else {
        bail!(ErrorKind::ConfigNotFound(path));
    }
}
