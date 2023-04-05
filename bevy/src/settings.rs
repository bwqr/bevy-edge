use bevy_ecs::system::Resource;
use serde::Deserialize;

#[derive(Deserialize)]
pub enum PhysicsPlugin {
    Default,
    Custom,
}

#[derive(Deserialize, Resource)]
pub struct Settings {
    pub physics_plugin: PhysicsPlugin,
    pub scene: String,
    pub num_object: usize,
}
