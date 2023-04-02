use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets};
use bevy_core::CorePlugin;
use bevy_core_pipeline::{prelude::Camera3dBundle, CorePipelinePlugin};
use bevy_ecs::{
    prelude::Component,
    system::{Commands, ResMut},
};
use bevy_input::InputPlugin;
use bevy_log::LogPlugin;
use bevy_math::Vec3;
use bevy_pbr::{AmbientLight, PbrBundle, PbrPlugin, StandardMaterial};
use bevy_rapier3d::prelude::{Collider, ColliderMassProperties, RigidBody};
use bevy_render::{
    prelude::{Color, Mesh},
    texture::ImagePlugin,
    RenderPlugin,
};
use bevy_scene::ScenePlugin;
use bevy_time::TimePlugin;
use bevy_transform::{prelude::Transform, TransformBundle, TransformPlugin};
use bevy_window::WindowPlugin;
use bevy_winit::WinitPlugin;

mod plugin;
mod request;
mod response;
mod server;
mod sync;
mod systems;

#[derive(Component)]
struct Shape;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "physics=debug");
    }

    let mut app = App::new();

    app.add_plugin(LogPlugin::default())
        .add_plugin(CorePlugin::default())
        .add_plugin(TimePlugin::default())
        .add_plugin(TransformPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default())
        .add_plugin(WinitPlugin::default())
        .add_plugin(RenderPlugin::default())
        .add_plugin(ImagePlugin::default())
        .add_plugin(CorePipelinePlugin::default())
        .add_plugin(PbrPlugin::default());

    app
        //.add_plugin(plugin::RapierPhysicsPlugin)
        .add_plugin(bevy_rapier3d::plugin::RapierPhysicsPlugin::<bevy_rapier3d::plugin::NoUserData>::default())
        //.add_plugin(bevy_rapier3d::render::RapierDebugRenderPlugin::default())
        ;

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
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
    } else {
        app.add_startup_system(balls);
    }

    app.add_system(bevy_window::close_on_esc);

    app.run()
}

fn balls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 50.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
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

    let num = 15;
    let rad = 1.5;

    let shift = rad * 2.0 + 1.0;
    let centerx = shift * (num as f32) / 2.0;
    let centery = shift / 2.0;
    let centerz = shift * (num as f32) / 2.0;

    for i in 0..num {
        for j in 0usize..num {
            for k in 0..num {
                let x = i as f32 * shift - centerx;
                let y = j as f32 * shift + centery;
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
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 50.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
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

    let num = 4;
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

    let num = 8;
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
