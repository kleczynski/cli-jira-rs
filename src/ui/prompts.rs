use crate::{
    io_utils::get_user_input,
    models::{Epic, Status, Story},
};

pub struct Prompts {
    pub create_epic: Box<dyn Fn() -> Epic>,
    pub create_story: Box<dyn Fn() -> Story>,
    pub delete_epic: Box<dyn Fn() -> bool>,
    pub delete_story: Box<dyn Fn() -> bool>,
    pub update_status: Box<dyn Fn() -> Option<Status>>,
}

impl Prompts {
    pub fn new() -> Self {
        Self {
            create_epic: Box::new(create_epic_prompt),
            create_story: Box::new(create_story_prompt),
            delete_epic: Box::new(delete_epic_prompt),
            delete_story: Box::new(delete_story_prompt),
            update_status: Box::new(update_status_prompt),
        }
    }
}

fn create_epic_prompt() -> Epic {
    println!("--------------------------");
    println!("Epic name:");
    let epic_name = get_user_input();
    println!("Epic description:");
    let epic_description = get_user_input();

    Epic::new(
        epic_name.trim().to_owned(),
        epic_description.trim().to_owned(),
    )
}

fn create_story_prompt() -> Story {
    println!("-------------------------");
    println!("Story name:");
    let story_name = get_user_input();

    println!("Story Description:");
    let story_description = get_user_input();

    Story::new(
        story_name.trim().to_owned(),
        story_description.trim().to_owned(),
    )
}

fn delete_epic_prompt() -> bool {
    println!("------------------------");
    println!("Are you sure you want to delete this epic? All stories in this epic will also be deleted [Y/n]:");
    let user_input = get_user_input();

    if user_input.trim().eq("Y") {
        return true;
    }

    false
}

fn delete_story_prompt() -> bool {
    println!("-----------------------");
    println!("Are you sure you want to delete this story? [Y/n]");

    let user_input = get_user_input();

    if user_input.trim().eq("Y") {
        return true;
    }

    false
}

fn update_status_prompt() -> Option<Status> {
    println!("----------------------");
    println!("New Status (1 - OPEN, 2 - IN PROGRESS, 3 - RESOLVED, 4 - CLOSED):");

    let new_status = get_user_input();
    let new_status = new_status.trim().parse::<u8>();
    if let Ok(status) = new_status {
        match status {
            1 => {
                return Some(Status::Open);
            }
            2 => {
                return Some(Status::InProgress);
            }
            3 => {
                return Some(Status::Resolved);
            }
            4 => {
                return Some(Status::Closed);
            }
            _ => return None,
        }
    }
    None
}
