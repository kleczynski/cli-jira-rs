use std::rc::Rc;

use anyhow::anyhow;
use anyhow::Result;
use itertools::Itertools;

use crate::db::JiraDatabase;
use crate::models::Action;
use std::any::Any;
mod page_helper;
use page_helper::*;

pub trait Page: Any {
    fn draw_page(&self) -> Result<()>;
    fn handle_input(&self, input: &str) -> Result<Option<Action>>;
    fn as_any(&self) -> &dyn Any;
}

impl dyn Page {
    fn downcast_ref<T: Any>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}

pub struct HomePage {
    pub db: Rc<JiraDatabase>,
}
impl Page for HomePage {
    fn draw_page(&self) -> Result<()> {
        println!("----------------------------- EPICS -----------------------------");
        println!("     id     |               name               |      status      ");
        let state = self.db.read_db()?;
        let epics: Vec<_> = state.epics.iter().sorted().collect();

        for (id, epic) in epics {
            let id_str = get_column_string(&id.to_string(), 12);
            let name_str = get_column_string(&epic.name, 32);
            let status_str = get_column_string(&format!("{:?}", &epic.status), 18);

            println!("{id_str}|{name_str}|{status_str}");
        }
        println!();
        println!();

        println!("[q] quit | [c] create epic | [:id:] navigate to epic");

        Ok(())
    }

    fn handle_input(&self, input: &str) -> Result<Option<Action>> {
        let epics = self.db.read_db()?.epics;

        match input {
            "q" => Ok(Some(Action::Exit)),
            "c" => Ok(Some(Action::CreateEpic)),
            input => {
                if let Ok(epic_id) = input.parse::<u32>() {
                    if epics.contains_key(&epic_id) {
                        return Ok(Some(Action::NavigateToEpicDetail { epic_id }));
                    }
                }
                Ok(None)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct EpicDetail {
    pub epic_id: u32,
    pub db: Rc<JiraDatabase>,
}

impl Page for EpicDetail {
    fn draw_page(&self) -> Result<()> {
        let db_state = self.db.read_db()?;
        let epic = db_state
            .epics
            .get(&self.epic_id)
            .ok_or_else(|| anyhow!("could not find epic!"))?;

        println!("------------------------------ EPIC ------------------------------");
        println!("  id  |     name     |         description         |    status    ");

        let id_str = get_column_string(&self.epic_id.to_string(), 8);
        let name_str = get_column_string(&epic.name, 12);
        let description_str = get_column_string(&epic.description, 18);
        let status_str = get_column_string(&epic.status.to_string(), 12);

        println!("{id_str}|{name_str}|{description_str}|{status_str}");

        println!();

        println!("---------------------------- STORIES ----------------------------");
        println!("     id     |               name               |      status      ");

        let stories = &db_state.stories;

        for (id, story) in stories {
            let id_str = get_column_string(&id.to_string(), 12);
            let name_str = get_column_string(&story.name, 32);
            let status_str = get_column_string(&story.status.to_string(), 18);

            println!("{id_str}|{name_str}|{status_str}");
        }

        println!();
        println!();

        println!("[p] previous | [u] update epic | [d] delete epic | [c] create story | [:id:] navigate to story");

        Ok(())
    }

    fn handle_input(&self, input: &str) -> Result<Option<Action>> {
        let stories = self.db.read_db()?.stories;

        match input {
            "p" => Ok(Some(Action::NavigateToPreviousPage)),
            "u" => Ok(Some(Action::UpdateEpicStatus {
                epic_id: self.epic_id,
            })),
            "d" => Ok(Some(Action::DeleteEpic {
                epic_id: self.epic_id,
            })),
            "c" => Ok(Some(Action::CreateStory {
                epic_id: self.epic_id,
            })),
            input => {
                if let Ok(story_id) = input.parse::<u32>() {
                    if stories.contains_key(&story_id) {
                        return Ok(Some(Action::NavigateToStoryDetail {
                            epic_id: self.epic_id,
                            story_id,
                        }));
                    }
                }
                Ok(None)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct StoryDetail {
    pub epic_id: u32,
    pub story_id: u32,
    pub db: Rc<JiraDatabase>,
}

impl Page for StoryDetail {
    fn draw_page(&self) -> Result<()> {
        let db_state = self.db.read_db()?;
        let story = db_state
            .stories
            .get(&self.story_id)
            .ok_or_else(|| anyhow!("could not find story!"))?;

        println!("------------------------------ STORY ------------------------------");
        println!("  id  |     name     |         description         |    status    ");

        let id_str = get_column_string(&self.story_id.to_string(), 8);
        let name_str = get_column_string(&story.name, 12);
        let description_str = get_column_string(&story.description, 24);
        let status_str = get_column_string(&story.status.to_string(), 12);

        println!("{id_str}|{name_str}|{description_str}|{status_str}");
        println!();
        println!();

        println!("[p] previous | [u] update story | [d] delete story");

        Ok(())
    }

    fn handle_input(&self, input: &str) -> Result<Option<Action>> {
        match input {
            "p" => Ok(Some(Action::NavigateToPreviousPage)),
            "u" => Ok(Some(Action::UpdateStoryStatus {
                story_id: self.story_id,
            })),
            "d" => Ok(Some(Action::DeleteStory {
                epic_id: self.epic_id,
                story_id: self.story_id,
            })),
            _ => Ok(None),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::MockDB;
    use crate::models::{Epic, Story};

    mod home_page {
        use super::*;

        #[test]
        fn draw_page_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let page = HomePage { db };
            assert!(page.draw_page().is_ok());
        }

        #[test]
        fn handle_input_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let page = HomePage { db };
            assert!(page.handle_input("").is_ok());
        }

        #[test]
        fn handle_input_should_return_the_correct_actions() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic = Epic::new("".to_owned(), "".to_owned());

            let epic_id = db.create_epic(epic).unwrap();

            let page = HomePage { db };

            let q = "q";
            let c = "c";
            let valid_epic_id = epic_id.to_string();
            let invalid_epic_id = "999";
            let junk_input = "j983f2j";
            let junk_input_with_valid_prefix = "q983f2j";
            let input_with_trailing_white_spaces = "q\n";

            assert!(matches!(page.handle_input(q).unwrap(), Some(Action::Exit)));
            assert!(matches!(
                page.handle_input(c).unwrap(),
                Some(Action::CreateEpic)
            ));
            assert!(matches!(
                page.handle_input(&valid_epic_id).unwrap(),
                Some(Action::NavigateToEpicDetail { epic_id: 1 })
            ));
            assert!(page.handle_input(invalid_epic_id).unwrap().is_none());
            assert!(page.handle_input(junk_input).unwrap().is_none());
            assert!(page
                .handle_input(junk_input_with_valid_prefix)
                .unwrap()
                .is_none());
            assert!(page
                .handle_input(input_with_trailing_white_spaces)
                .unwrap()
                .is_none());
        }
    }

    mod epic_detail_page {
        use super::*;

        #[test]
        fn draw_page_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });
            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();

            let page = EpicDetail { epic_id, db };
            assert!(page.draw_page().is_ok());
        }

        #[test]
        fn handle_input_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });
            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();

            let page = EpicDetail { epic_id, db };
            assert!(page.handle_input("").is_ok());
        }

        #[test]
        fn draw_page_should_throw_error_for_invalid_epic_id() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let page = EpicDetail { epic_id: 999, db };
            assert!(page.draw_page().is_err());
        }

        #[test]
        fn handle_input_should_return_the_correct_actions() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();
            let story_id = db
                .create_story(Story::new("".to_owned(), "".to_owned()), epic_id)
                .unwrap();

            let page = EpicDetail { epic_id, db };

            let p = "p";
            let u = "u";
            let d = "d";
            let c = "c";
            let invalid_story_id = "999";
            let junk_input = "j983f2j";
            let junk_input_with_valid_prefix = "p983f2j";
            let input_with_trailing_white_spaces = "p\n";

            assert!(matches!(
                page.handle_input(p).unwrap(),
                Some(Action::NavigateToPreviousPage)
            ));
            assert!(matches!(
                page.handle_input(u).unwrap(),
                Some(Action::UpdateEpicStatus { epic_id: 1 })
            ));
            assert!(matches!(
                page.handle_input(d).unwrap(),
                Some(Action::DeleteEpic { epic_id: 1 })
            ));
            assert!(matches!(
                page.handle_input(c).unwrap(),
                Some(Action::CreateStory { epic_id: 1 })
            ));
            assert!(matches!(
                page.handle_input(&story_id.to_string()).unwrap(),
                Some(Action::NavigateToStoryDetail {
                    epic_id: 1,
                    story_id: 2
                })
            ));
            assert!(page.handle_input(invalid_story_id).unwrap().is_none());
            assert!(page.handle_input(junk_input).unwrap().is_none());
            assert!(page
                .handle_input(junk_input_with_valid_prefix)
                .unwrap()
                .is_none());
            assert!(page
                .handle_input(input_with_trailing_white_spaces)
                .unwrap()
                .is_none());
        }
    }

    mod story_detail_page {
        use super::*;

        #[test]
        fn draw_page_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();
            let story_id = db
                .create_story(Story::new("".to_owned(), "".to_owned()), epic_id)
                .unwrap();

            let page = StoryDetail {
                epic_id,
                story_id,
                db,
            };
            assert!(page.draw_page().is_ok());
        }

        #[test]
        fn handle_input_should_not_throw_error() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();
            let story_id = db
                .create_story(Story::new("".to_owned(), "".to_owned()), epic_id)
                .unwrap();

            let page = StoryDetail {
                epic_id,
                story_id,
                db,
            };
            assert!(page.handle_input("").is_ok());
        }

        #[test]
        fn draw_page_should_throw_error_for_invalid_story_id() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();
            let _ = db
                .create_story(Story::new("".to_owned(), "".to_owned()), epic_id)
                .unwrap();

            let page = StoryDetail {
                epic_id,
                story_id: 999,
                db,
            };
            assert!(page.draw_page().is_err());
        }

        #[test]
        fn handle_input_should_return_the_correct_actions() {
            let db = Rc::new(JiraDatabase {
                database: Box::new(MockDB::new()),
            });

            let epic_id = db
                .create_epic(Epic::new("".to_owned(), "".to_owned()))
                .unwrap();
            let story_id = db
                .create_story(Story::new("".to_owned(), "".to_owned()), epic_id)
                .unwrap();

            let page = StoryDetail {
                epic_id,
                story_id,
                db,
            };

            let p = "p";
            let u = "u";
            let d = "d";
            let some_number = "1";
            let junk_input = "j983f2j";
            let junk_input_with_valid_prefix = "p983f2j";
            let input_with_trailing_white_spaces = "p\n";

            assert!(matches!(
                page.handle_input(p).unwrap(),
                Some(Action::NavigateToPreviousPage)
            ));
            assert!(matches!(
                page.handle_input(u).unwrap(),
                Some(Action::UpdateStoryStatus { story_id: _ })
            ));
            assert!(matches!(
                page.handle_input(d).unwrap(),
                Some(Action::DeleteStory {
                    epic_id: _,
                    story_id: _
                })
            ));
            assert!(page.handle_input(some_number).unwrap().is_none());
            assert!(page.handle_input(junk_input).unwrap().is_none());
            assert!(page
                .handle_input(junk_input_with_valid_prefix)
                .unwrap()
                .is_none());
            assert!(page
                .handle_input(input_with_trailing_white_spaces)
                .unwrap()
                .is_none());
        }
    }
}
