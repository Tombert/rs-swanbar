use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
pub struct Meta {
    pub is_processing: bool,
    pub start_time: Duration,
    pub data: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Out {
    pub name: String,
    pub instance: String,
    pub full_text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistConfig {
    pub path: String,
    pub buffer_size: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: String,
    pub ttl: u64,
    pub timeout: Option<u64>,
    pub display: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub poll_time: Option<u64>,
    pub default_timeout: u64,
    pub suspend_time: Option<u64>,
    pub persist: PersistConfig,
    pub modules: Vec<ModuleConfig>,
}
