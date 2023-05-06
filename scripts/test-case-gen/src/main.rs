use std::{fmt::Display, io::Write};

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
pub struct Scene {
    pub camera: (f32, f32, f32),
    pub num_object: usize,
    pub shape: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Settings {
    pub tracing_level: Option<String>,
    pub headless: bool,
    pub physics_plugin: PhysicsPlugin,
    pub bench_length: f32,
    pub scene: Scene,
}

fn main() {
    let output_dir = std::env::args().into_iter().collect::<Vec<String>>();
    let plugins = [
        PhysicsPlugin::Default,
        PhysicsPlugin::Server {
            compress: None,
            address: "192.168.1.240:4001".to_string(),
        },
        PhysicsPlugin::Server {
            compress: Some(2),
            address: "192.168.1.240:4001".to_string(),
        },
        PhysicsPlugin::Server {
            compress: Some(9),
            address: "192.168.1.240:4001".to_string(),
        },
    ];

    let num_objects = [500, 1000, 2000, 4000, 8000];

    let shapes = ["ball", "capsule", "cuboid"];

    for plugin in plugins {
        for num_object in num_objects {
            for shape in shapes {
                let mut file =
                    std::fs::File::create(format!("{}/{}_{}_{}", output_dir[1], plugin, num_object, shape)).unwrap();
                file.write(
                    ron::to_string(&Settings {
                        tracing_level: None,
                        headless: true,
                        physics_plugin: plugin.clone(),
                        bench_length: 10.0,
                        scene: Scene {
                            camera: (0.0, 80.0, 260.0),
                            num_object,
                            shape: shape.to_string(),
                        },
                    })
                    .unwrap()
                    .as_bytes(),
                )
                .unwrap();
            }
        }
    }
}
