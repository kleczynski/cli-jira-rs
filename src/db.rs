use std::{
    fs::{self, OpenOptions},
    io::Write,
};

use crate::models::{DBState, Epic, Status, Story};
use anyhow::{anyhow, Context, Result};

pub struct JiraDatabase {
    pub database: Box<dyn Database>,
}

impl JiraDatabase {
    pub fn new(file_path: String) -> Self {
        Self {
            database: Box::new(JSONFileDatabase { file_path }),
        }
    }
    pub fn read_db(&self) -> Result<DBState> {
        self.database.read_db()
    }

    pub fn create_epic(&self, epic: Epic) -> Result<u32> {
        let mut state = self.database.read_db()?;
        let last_item_id = state.last_item_id;
        let new_id = last_item_id + 1;
        state.last_item_id = new_id;
        state.epics.insert(new_id, epic);
        self.database.write_db(&state)?;
        Ok(state.last_item_id)
    }

    pub fn create_story(&self, story: Story, epic_id: u32) -> Result<u32> {
        let mut state = self.database.read_db()?;
        let last_item_id = state.last_item_id;
        let new_id = last_item_id + 1;
        state.last_item_id = new_id;
        state.stories.insert(new_id, story);
        let stories = state.epics.get_mut(&epic_id);
        stories
            .ok_or_else(|| anyhow!("Failed to get epic with id: {epic_id}"))?
            .stories
            .push(new_id);
        self.database.write_db(&state)?;
        Ok(state.last_item_id)
    }
    pub fn delete_epic(&self, epic_id: u32) -> Result<()> {
        let mut state = self.database.read_db()?;
        for story in &state
            .epics
            .get(&epic_id)
            .ok_or_else(|| anyhow!("Failed to get epic with id {}", &epic_id))?
            .stories
        {
            state.stories.remove(story);
        }
        state.epics.remove(&epic_id);
        self.database.write_db(&state)?;
        Ok(())
    }
    pub fn delete_story(&self, epic_id: u32, story_id: u32) -> Result<()> {
        let mut state = self.database.read_db()?;
        let epics = state
            .epics
            .get_mut(&epic_id)
            .ok_or_else(|| anyhow!("Failed to get epic with id {}", &epic_id))?;
        let story_index = epics
            .stories
            .iter()
            .position(|id| id == &story_id)
            .ok_or_else(|| {
                anyhow!(
                    "Failed to find a story with id {} in epic with id {}",
                    &story_id,
                    &epic_id
                )
            })?;
        epics.stories.remove(story_index);
        state.stories.remove(&story_id);

        self.database.write_db(&state)?;
        Ok(())
    }
    pub fn update_epic_status(&self, epic_id: u32, status: Status) -> Result<()> {
        let mut state = self.database.read_db()?;
        let epic = state
            .epics
            .get_mut(&epic_id)
            .ok_or_else(|| anyhow!("Failed to get epic with id {}", &epic_id))?;
        epic.status = status;
        self.database.write_db(&state)?;
        Ok(())
    }
    pub fn update_story_status(&self, story_id: u32, status: Status) -> Result<()> {
        let mut state = self.database.read_db()?;
        let story = state
            .stories
            .get_mut(&story_id)
            .ok_or_else(|| anyhow!("Failed to get story with id {}", &story_id))?;

        story.status = status;
        self.database.write_db(&state)?;
        Ok(())
    }
}

pub trait Database {
    fn read_db(&self) -> Result<DBState>;
    fn write_db(&self, db_state: &DBState) -> Result<()>;
}

#[derive(Debug)]
struct JSONFileDatabase {
    pub file_path: String,
}

impl Database for JSONFileDatabase {
    fn read_db(&self) -> Result<DBState> {
        let contents = fs::read_to_string(&self.file_path)
            .with_context(|| format!("Failed to open file {}", &self.file_path))?;
        let db_state: DBState = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to deserialised content: {}", &contents))?;

        Ok(db_state)
    }
    fn write_db(&self, db_state: &DBState) -> Result<()> {
        let state = serde_json::to_string(db_state)
            .with_context(|| format!("Failed to convert {db_state:?} to JSON"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .with_context(|| format!("Failed to open file with path {}", &self.file_path))?;

        file.write_all(state.as_bytes()).with_context(|| {
            format!(
                "Failed to write content {} to file {}",
                &state, &self.file_path
            )
        })?;
        Ok(())
    }
}

pub mod test_utils {
    use std::{cell::RefCell, collections::HashMap};

    use super::*;

    pub struct MockDB {
        last_written_state: RefCell<DBState>,
    }

    impl MockDB {
        pub fn new() -> Self {
            Self {
                last_written_state: RefCell::new(DBState {
                    last_item_id: 0,
                    epics: HashMap::new(),
                    stories: HashMap::new(),
                }),
            }
        }
    }

    impl Database for MockDB {
        fn read_db(&self) -> Result<DBState> {
            let state = self.last_written_state.borrow().clone();
            Ok(state)
        }

        fn write_db(&self, db_state: &DBState) -> Result<()> {
            let latest_state = &self.last_written_state;
            *latest_state.borrow_mut() = db_state.clone();
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_utils::MockDB;
    use super::*;

    #[test]
    fn create_epic_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic.clone());

        assert!(result.is_ok());

        let id = result.unwrap();
        let db_state = db.read_db().unwrap();

        let expected_id = 1;

        assert!(id == expected_id);
        assert!(db_state.last_item_id == expected_id);
        assert!(db_state.epics.get(&id) == Some(&epic));
    }
    #[test]
    fn create_story_should_error_if_invalid_epic_id() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let story = Story::new("".to_owned(), "".to_owned());

        let non_existent_epic_id = 999;

        let result = db.create_story(story, non_existent_epic_id);
        assert!(result.is_err());
    }

    #[test]
    fn create_story_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);
        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.create_story(story.clone(), epic_id);
        assert!(result.is_ok());

        let id = result.unwrap();
        let db_state = db.read_db().unwrap();

        let expected_id = 2;

        assert!(id == expected_id);
        assert!(db_state.last_item_id == expected_id);
        assert!(db_state.epics.get(&epic_id).unwrap().stories.contains(&id));
        assert!(db_state.stories.get(&id) == Some(&story));
    }

    #[test]
    fn delete_epic_should_error_if_invalid_epic_id() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };

        let non_existent_epic_id = 999;

        let result = db.delete_epic(non_existent_epic_id);
        assert!(result.is_err());
    }

    #[test]
    fn delete_epic_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);
        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.create_story(story, epic_id);
        assert!(result.is_ok());

        let story_id = result.unwrap();

        let result = db.delete_epic(epic_id);
        assert!(result.is_ok());

        let db_state = db.read_db().unwrap();

        let expected_last_id = 2;

        assert!(db_state.last_item_id == expected_last_id);
        assert!(!db_state.epics.contains_key(&epic_id));
        assert!(!db_state.stories.contains_key(&story_id));
    }

    #[test]
    fn delete_story_should_error_if_invalid_epic_id() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);
        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.create_story(story, epic_id);
        assert!(result.is_ok());

        let story_id = result.unwrap();

        let non_existent_epic_id = 999;

        let result = db.delete_story(non_existent_epic_id, story_id);
        assert!(result.is_err());
    }

    #[test]
    fn delete_story_should_error_if_story_not_found_in_epic() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);
        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.create_story(story, epic_id);
        assert!(result.is_ok());

        let non_existent_story_id = 999;

        let result = db.delete_story(epic_id, non_existent_story_id);
        assert!(result.is_err());
    }

    #[test]
    fn delete_story_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);
        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.create_story(story, epic_id);
        assert!(result.is_ok());

        let story_id = result.unwrap();

        let result = db.delete_story(epic_id, story_id);
        assert!(result.is_ok());

        let db_state = db.read_db().unwrap();

        let expected_last_id = 2;

        assert!(db_state.last_item_id == expected_last_id);
        assert!(!db_state
            .epics
            .get(&epic_id)
            .unwrap()
            .stories
            .contains(&story_id));
        assert!(!db_state.stories.contains_key(&story_id));
    }

    #[test]
    fn update_epic_status_should_error_if_invalid_epic_id() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };

        let non_existent_epic_id = 999;

        let result = db.update_epic_status(non_existent_epic_id, Status::Closed);
        assert!(result.is_err());
    }

    #[test]
    fn update_epic_status_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);

        assert!(result.is_ok());

        let epic_id = result.unwrap();

        let result = db.update_epic_status(epic_id, Status::Closed);

        assert!(result.is_ok());

        let db_state = db.read_db().unwrap();

        assert!(matches!(
            db_state.epics.get(&epic_id).unwrap().status,
            Status::Closed
        ));
    }

    #[test]
    fn update_story_status_should_error_if_invalid_story_id() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };

        let non_existent_story_id = 999;

        let result = db.update_story_status(non_existent_story_id, Status::Closed);
        assert!(result.is_err());
    }

    #[test]
    fn update_story_status_should_work() {
        let db = JiraDatabase {
            database: Box::new(MockDB::new()),
        };
        let epic = Epic::new("".to_owned(), "".to_owned());
        let story = Story::new("".to_owned(), "".to_owned());

        let result = db.create_epic(epic);

        let epic_id = result.unwrap();

        let result = db.create_story(story, epic_id);

        let story_id = result.unwrap();

        let result = db.update_story_status(story_id, Status::Closed);

        assert!(result.is_ok());

        let db_state = db.read_db().unwrap();

        assert!(matches!(
            db_state.stories.get(&story_id).unwrap().status,
            Status::Closed
        ));
    }

    mod database {
        use std::collections::HashMap;
        use std::io::Write;

        use super::*;

        #[test]
        fn read_db_should_fail_with_invalid_path() {
            let db = JSONFileDatabase {
                file_path: "INVALID_PATH".to_owned(),
            };
            assert!(db.read_db().is_err());
        }

        #[test]
        fn read_db_should_fail_with_invalid_json() {
            let mut tmpfile = tempfile::NamedTempFile::new().unwrap();

            let file_contents = r#"{ "last_item_id": 0 epics: {} stories {} }"#;
            write!(tmpfile, "{}", file_contents).unwrap();

            let db = JSONFileDatabase {
                file_path: tmpfile
                    .path()
                    .to_str()
                    .expect("failed to convert tmpfile path to str")
                    .to_string(),
            };

            let result = db.read_db();

            assert!(result.is_err());
        }

        #[test]
        fn read_db_should_parse_json_file() {
            let mut tmpfile = tempfile::NamedTempFile::new().unwrap();

            let file_contents = r#"{ "last_item_id": 0, "epics": {}, "stories": {} }"#;
            write!(tmpfile, "{}", file_contents).unwrap();

            let db = JSONFileDatabase {
                file_path: tmpfile
                    .path()
                    .to_str()
                    .expect("failed to convert tmpfile path to str")
                    .to_string(),
            };

            let result = db.read_db();

            assert!(result.is_ok());
        }

        #[test]
        fn write_db_should_work() {
            let mut tmpfile = tempfile::NamedTempFile::new().unwrap();

            let file_contents = r#"{ "last_item_id": 0, "epics": {}, "stories": {} }"#;
            write!(tmpfile, "{}", file_contents).unwrap();

            let db = JSONFileDatabase {
                file_path: tmpfile
                    .path()
                    .to_str()
                    .expect("failed to convert tmpfile path to str")
                    .to_string(),
            };

            let story = Story {
                name: "epic 1".to_owned(),
                description: "epic 1".to_owned(),
                status: Status::Open,
            };
            let epic = Epic {
                name: "epic 1".to_owned(),
                description: "epic 1".to_owned(),
                status: Status::Open,
                stories: vec![2],
            };

            let mut stories = HashMap::new();
            stories.insert(2, story);

            let mut epics = HashMap::new();
            epics.insert(1, epic);

            let state = DBState {
                last_item_id: 2,
                epics,
                stories,
            };

            let write_result = db.write_db(&state);
            let read_result = db.read_db().unwrap();

            assert!(write_result.is_ok());
            assert!(read_result == state);
        }
    }
}
