use bevy_ecs::system::Resource;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub enum PhysicsPlugin {
    Default,
    Server {
        compress: Option<u32>,
        address: String,
    },
}

#[derive(Clone, Deserialize, Resource)]
pub struct Settings {
    pub tracing_level: Option<String>,
    pub headless: bool,
    pub physics_plugin: PhysicsPlugin,
    pub scene: String,
    pub num_object: usize,
}
