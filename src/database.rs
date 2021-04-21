use std::{
    path::{Path},
    fs::{File},
};
use serde_json;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Database {
    tasks: Vec<Task>,
}

#[serde(default)]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Task {
    text: String,
    seconds_estimated: i64,
    seconds_spent: i64,
    done: bool,
}

pub fn load_data(filename: &str) -> Database {
    let file = File::open(filename).unwrap();
    let deserialized: Database = serde_json::from_reader(&file).unwrap();
    deserialized
}

pub fn save_data(filename: &str, data: Database) {
    let file = File::create(filename).unwrap();
    serde_json::to_writer(&file, &data).unwrap();
}

pub fn default_data() -> Database {
    let tasks: Vec<Task> = vec![
        Task {
            text: String::from("Task 1"),
            seconds_estimated: 30 * 60,
            seconds_spent: 0,
            done: false,
        },
        Task {
            text: String::from("Task 2"),
            seconds_estimated: 30 * 60,
            seconds_spent: 0,
            done: false,
        }
    ];
    Database { tasks }
}
