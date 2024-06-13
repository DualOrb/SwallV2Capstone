use serde::{Deserialize, Serialize};
use std::process::Child;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompositorConfig {
    pub width: u32,
    pub height: u32,
    pub launch: Vec<AppConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub executable: String,
    pub args: Vec<String>,
    pub area: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn is_inside(&self, x: u32, y: u32) -> bool {
        self.x < x && x < (self.x + self.width) && self.y < y && y < (self.y + self.height)
    }
}

#[derive(Debug)]
pub struct CompositorProcess {
    pub child: Child,
    pub config: AppConfig,
}

/// Represents commands issues by the app controller to the compositor
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum AppControllerCommand {
    Spawn { config: AppConfig },
    Move { pid: u32, rect: Rect },
    Kill { pid: u32 },
    List,
    ScreenSize,
}
