use super::{app_data_directory, AppFile};
use crate::api::api_client;
use crate::api::{
    AbilityFetcher, EvolutionFetcher, FetchResource, GameFetcher, MoveFetcher, PokemonFetcher,
    SpeciesFetcher, TypeFetcher,
};
use crate::models::resource::{InsertRow, MetaRow, SelectRow};
use crate::VERSION;

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Result};
use rusqlite::{Connection, OpenFlags, Result as SqlResult};
use rustemon::client::RustemonClient;
use semver::Version;

pub struct DatabaseFile {
    path: PathBuf,
}
impl Default for DatabaseFile {
    fn default() -> Self {
        Self { path: Self::path() }
    }
}
impl DatabaseFile {
    pub fn connect(&self) -> Result<Connection> {
        let mut flags = OpenFlags::default();
        flags.set(OpenFlags::SQLITE_OPEN_READ_WRITE, false);
        flags.set(OpenFlags::SQLITE_OPEN_CREATE, false);
        flags.set(OpenFlags::SQLITE_OPEN_READ_ONLY, true);

        let open = Connection::open_with_flags(&self.path, flags);
        if let Ok(db) = open {
            return Self::version_check(db);
        }

        bail!("Database not set up. Run `dunspars setup` first.")
    }

    fn version_check(db: Connection) -> Result<Connection> {
        let meta = MetaRow::select_by_name("version", &db);

        if let Ok(db_version) = meta {
            if versions_within_minor_level(&db_version.value, VERSION).unwrap_or(false) {
                return Ok(db);
            }

            bail!(
                "Database version mismatch. Program version: {0}; Database version: {1}. Run `dunspars setup` again.",
                VERSION,
                db_version.value
            )
        }

        bail!("Database malformed. Run `dunspars setup` again.")
    }

    pub async fn build_db(&self, writer: &mut impl std::io::Write) -> Result<()> {
        Self::build_dir()?;
        if Self::path_exists(&self.path) {
            fs::remove_file(&self.path)?;
        }

        let api = api_client();
        let mut db = Connection::open(&self.path)?;

        let start = std::time::Instant::now();

        self.create_schema(&db)?;

        // Games must always be retrieved first as game-to-generation
        // conversion data is needed for the other tables.
        writeln!(writer, "retrieving games")?;
        self.fetch_and_populate::<GameFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving moves")?;
        self.fetch_and_populate::<MoveFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving types")?;
        self.fetch_and_populate::<TypeFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving abilities")?;
        self.fetch_and_populate::<AbilityFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving species")?;
        self.fetch_and_populate::<SpeciesFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving evolution")?;
        self.fetch_and_populate::<EvolutionFetcher>(&api, &mut db)
            .await?;

        writeln!(writer, "retrieving pokemon")?;
        self.fetch_and_populate::<PokemonFetcher>(&api, &mut db)
            .await?;

        self.populate_meta(&mut db)?;

        let duration = start.elapsed();
        writeln!(writer, "setup time: {}s", duration.as_secs())?;

        Ok(())
    }

    fn create_schema(&self, db: &Connection) -> SqlResult<()> {
        db.execute_batch(include_str!("../sql/create_schema.sql"))
    }

    async fn fetch_and_populate<T: FetchResource>(
        &self,
        api: &RustemonClient,
        db: &mut Connection,
    ) -> Result<()> {
        let rows = T::fetch_resource(api, db).await?;
        self.populate_table(rows, db)?;
        Ok(())
    }

    fn populate_table(&self, entries: Vec<impl InsertRow>, db: &mut Connection) -> SqlResult<()> {
        let transaction = db.transaction()?;
        for entry in entries {
            entry.insert(&transaction)?;
        }
        transaction.commit()
    }

    fn populate_meta(&self, db: &mut Connection) -> SqlResult<()> {
        let meta = vec![MetaRow {
            name: String::from("version"),
            value: String::from(VERSION),
        }];
        self.populate_table(meta, db)
    }
}
impl AppFile for DatabaseFile {
    fn path() -> PathBuf {
        app_data_directory("resource.db")
    }
}

fn versions_within_minor_level(lhs: &str, rhs: &str) -> Result<bool> {
    let left = Version::parse(lhs)?;
    let right = Version::parse(rhs)?;

    if left.major == right.major && left.minor == right.minor {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_meet_criteria() {
        let same_major_minor = versions_within_minor_level("1.2.3", "1.2.0").unwrap();
        assert!(same_major_minor);

        let different_major = versions_within_minor_level("1.2.0", "2.2.0").unwrap();
        assert!(!different_major);

        let different_minor = versions_within_minor_level("1.2.2", "1.3.2").unwrap();
        assert!(!different_minor);

        let parse_error = versions_within_minor_level("1.2.3", "1.23");
        assert!(parse_error.is_err());
    }
}
