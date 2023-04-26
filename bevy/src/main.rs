use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets, AddAsset};
use bevy_core::CorePlugin;
use bevy_core_pipeline::{prelude::Camera3dBundle, CorePipelinePlugin};
use bevy_ecs::{
    prelude::Component,
    system::{Commands, ResMut, Res},
};
use bevy_input::InputPlugin;
use bevy_log::LogPlugin;
use bevy_math::Vec3;
use bevy_pbr::{AmbientLight, PbrBundle, PbrPlugin, StandardMaterial};
use bevy_rapier3d::prelude::{Collider, ColliderMassProperties, RigidBody};
use bevy_render::{
    prelude::{Color, Mesh},
    texture::ImagePlugin,
    RenderPlugin, render_resource::{Shader, ShaderLoader}, mesh::MeshPlugin,
};
use bevy_scene::ScenePlugin;
use bevy_time::{TimePlugin, Time};
use bevy_transform::{prelude::Transform, TransformBundle, TransformPlugin};
use bevy_window::WindowPlugin;
use bevy_winit::WinitPlugin;
use shared::settings::{Settings, PhysicsPlugin};

mod physics;

#[derive(Component)]
struct Shape;

fn main() {
    let settings_path = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .map(|s| s.to_owned())
        .unwrap_or("Settings.ron".to_string());

    let settings: Settings = ron::de::from_reader(std::fs::File::open(settings_path).unwrap()).unwrap();

    let mut app = App::new();

    if let Some(level) = &settings.tracing_level {
        let tracing_level = match level.as_str() {
            "DEBUG" => bevy_utils::tracing::Level::DEBUG,
            "INFO" => bevy_utils::tracing::Level::INFO,
            "ERROR" => bevy_utils::tracing::Level::ERROR,
            _ => bevy_utils::tracing::Level::WARN,
        };

        app.add_plugin(LogPlugin { level: tracing_level, ..Default::default() });
    }

    app
        .add_plugin(CorePlugin::default())
        .add_plugin(TimePlugin::default())
        .add_plugin(TransformPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default())
        .add_plugin(WinitPlugin::default());

    if settings.headless {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        app.add_plugin(MeshPlugin);
    } else {
        app.add_plugin(RenderPlugin::default());
    }

    app
        .add_plugin(ImagePlugin::default())
        .add_plugin(CorePipelinePlugin::default())
        .add_plugin(PbrPlugin::default());

    match &settings.physics_plugin {
        PhysicsPlugin::Default => {
            app.add_plugin(bevy_rapier3d::plugin::RapierPhysicsPlugin::<bevy_rapier3d::plugin::NoUserData>::default());
        }
        PhysicsPlugin::Server { address, compress } => {
            app.add_plugin(physics::RapierPhysicsPlugin { address: address.clone(), compress: *compress });
        }
    }

    match settings.scene.as_str() {
        "boxes" => {
            app.add_startup_system(boxes);
        }
        "capsules" => {
            app.add_startup_system(capsules);
        }
        _ => {
            app.add_startup_system(balls);
        }
    }

    app.add_system(bevy_window::close_on_esc)
        .add_system(log)
        .insert_resource(settings);

    app.run()
}

fn balls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 75.0, 200.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(100.0, 0.1, 100.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -2.0, 0.0)))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Box::new(100.0, 0.1, 100.0).into()),
            material: materials.add(Color::BLACK.into()),
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..Default::default()
        });

    let num = 10;
    let rad = 1.5;

    let shift = rad * 2.0 + 1.0;
    let centerx = shift * (num as f32) / 2.0;
    let centery = shift / 2.0;
    let centerz = shift * (num as f32) / 2.0;

    for i in 0..num {
        for j in 0usize..(settings.num_object / num / num) {
            for k in 0..num {
                let x = i as f32 * shift - centerx;
                let y = j as f32 * shift * 2.0 + centery;
                let z = k as f32 * shift - centerz;

                let status = if j == 0 {
                    RigidBody::Fixed
                } else {
                    RigidBody::Dynamic
                };

                let density = 0.477;

                commands
                    .spawn(status)
                    .insert(Collider::ball(rad))
                    .insert(ColliderMassProperties::Density(density))
                    .insert(TransformBundle::from(Transform::from_xyz(x, y, z)))
                    .insert(Shape)
                    .insert(PbrBundle {
                        mesh: meshes.add(
                            bevy_render::prelude::shape::Icosphere {
                                radius: rad,
                                ..Default::default()
                            }
                            .into(),
                        ),
                        material: materials.add(Color::YELLOW.into()),
                        transform: Transform::from_xyz(x, y, z),
                        ..Default::default()
                    });
            }
        }
    }

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });
}

fn boxes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 50.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(100.0, 0.1, 100.0))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Box::new(100.0, 0.1, 100.0).into()),
            material: materials.add(Color::BLACK.into()),
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..Default::default()
        });

    let num = settings.num_object;
    let rad = 2.0;

    let shift = rad * 2.0 + rad;
    let centerx = shift * (num / 2) as f32;
    let centery = shift / 2.0;
    let centerz = shift * (num / 2) as f32;

    let mut offset = -(num as f32) * (rad * 2.0 + rad) * 0.5;

    for j in 0usize..47 {
        for i in 0..num {
            for k in 0usize..num {
                let x = i as f32 * shift - centerx + offset;
                let y = j as f32 * shift + centery + 3.0;
                let z = k as f32 * shift - centerz + offset;

                let status = RigidBody::Dynamic;

                commands
                    .spawn(status)
                    .insert(Collider::cuboid(rad, rad, rad))
                    .insert(Shape)
                    .insert(PbrBundle {
                        mesh: meshes.add(bevy_render::prelude::shape::Cube::new(rad * 2.0).into()),
                        material: materials.add(Color::YELLOW.into()),
                        transform: Transform::from_xyz(x, y, z),
                        ..Default::default()
                    });
            }
        }

        offset -= 0.05 * rad * (num as f32 - 1.0);
    }

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });
}

fn capsules(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 50.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(100.0, 0.1, 100.0))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Box::new(200.0, 0.2, 200.0).into()),
            material: materials.add(Color::BLACK.into()),
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..Default::default()
        });

    let num = settings.num_object;
    let rad = 1.0;

    let shift = rad * 2.0 + rad;
    let shifty = rad * 4.0;
    let centerx = shift * (num / 2) as f32;
    let centery = shift / 2.0;
    let centerz = shift * (num / 2) as f32;

    let mut offset = -(num as f32) * (rad * 2.0 + rad) * 0.5;

    for j in 0usize..47 {
        for i in 0..num {
            for k in 0usize..num {
                let x = i as f32 * shift - centerx + offset;
                let y = j as f32 * shifty + centery + 3.0;
                let z = k as f32 * shift - centerz + offset;

                let status = RigidBody::Dynamic;

                commands
                    .spawn(status)
                    .insert(Collider::capsule_y(rad, rad))
                    .insert(Shape)
                    .insert(PbrBundle {
                        mesh: meshes.add(
                            bevy_render::prelude::shape::Capsule {
                                radius: rad,
                                depth: rad * 2.0,
                                ..Default::default()
                            }
                            .into(),
                        ),
                        material: materials.add(Color::YELLOW.into()),
                        transform: Transform::from_xyz(x, y, z),
                        ..Default::default()
                    });
            }
        }

        offset -= 0.05 * rad * (num as f32 - 1.0);
    }

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.5,
    });
}

fn log(time: Res<Time>) {
    println!("FPS {}", 1.0 / time.delta_seconds());
}
