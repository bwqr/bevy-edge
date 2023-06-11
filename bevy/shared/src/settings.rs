use std::fmt::Display;

use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub enum PhysicsPlugin {
    Default,
    Server {
        compress: Option<u32>,
        address: String,
    },
}

impl Display for PhysicsPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsPlugin::Default => f.write_str("default"),
            PhysicsPlugin::Server { compress: None, .. } => f.write_str("server_none"),
            PhysicsPlugin::Server {
                compress: Some(compress),
                ..
            } => f.write_str(format!("server_{}", compress).as_str()),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Room {
    Open,
    Closed,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Scene {
    pub camera: (f32, f32, f32),
    pub num_object: usize,
    pub shape: String,
    pub restitution: f32,
    pub room: Room,
    pub ccd: bool,
}

#[derive(Clone, Deserialize, Resource, Serialize)]
pub struct Settings {
    pub tracing_level: Option<String>,
    pub headless: bool,
    pub physics_plugin: PhysicsPlugin,
    pub bench_length: f32,
    pub scene: Scene,
}

impl Display for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}_{}_{}",  self.physics_plugin, self.scene.num_object, self.scene.shape))
    }
}
