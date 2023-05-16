use bevy_app::App;
use bevy_asset::{AddAsset, AssetPlugin, Assets};
use bevy_core::CorePlugin;
use bevy_core_pipeline::{prelude::Camera3dBundle, CorePipelinePlugin};
use bevy_ecs::{
    prelude::Component,
    system::{Commands, Res, ResMut},
};
use bevy_input::InputPlugin;
use bevy_log::LogPlugin;
use bevy_math::Vec3;
use bevy_pbr::{AmbientLight, PbrBundle, PbrPlugin, StandardMaterial};
use bevy_rapier3d::{prelude::{Collider, ColliderMassProperties, RigidBody, Velocity}, rapier::prelude::{Isometry, SharedShape}};
use bevy_render::{
    mesh::MeshPlugin,
    prelude::{Color, Mesh},
    render_resource::{Shader, ShaderLoader},
    texture::ImagePlugin,
    RenderPlugin,
};
use bevy_scene::ScenePlugin;
use bevy_time::TimePlugin;
use bevy_transform::{prelude::Transform, TransformBundle, TransformPlugin};
use bevy_window::WindowPlugin;
use bevy_winit::WinitPlugin;
use shared::settings::{PhysicsPlugin, Settings};

mod bench;
mod physics;

#[derive(Component)]
struct Shape;

fn main() {
    env_logger::init();

    let settings_path = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .map(|s| s.to_owned())
        .unwrap_or("Settings.ron".to_string());

    let settings: Settings =
        ron::de::from_reader(std::fs::File::open(settings_path).unwrap()).unwrap();

    let mut app = App::new();

    if let Some(level) = &settings.tracing_level {
        let tracing_level = match level.as_str() {
            "DEBUG" => bevy_utils::tracing::Level::DEBUG,
            "INFO" => bevy_utils::tracing::Level::INFO,
            "ERROR" => bevy_utils::tracing::Level::ERROR,
            _ => bevy_utils::tracing::Level::WARN,
        };

        app.add_plugin(LogPlugin {
            level: tracing_level,
            ..Default::default()
        });
    }

    // For scripting purposes, we run the event loop even the window get unfocused.
    app.insert_resource(bevy_winit::WinitSettings {
        unfocused_mode: bevy_winit::UpdateMode::Continuous,
        focused_mode: bevy_winit::UpdateMode::Continuous,
        ..Default::default()
    });

    app.add_plugin(CorePlugin::default())
        .add_plugin(TimePlugin::default())
        .add_plugin(TransformPlugin::default())
        .add_plugin(InputPlugin::default())
        .add_plugin(WindowPlugin {
            window: bevy_window::WindowDescriptor { present_mode: bevy_window::PresentMode::AutoNoVsync, ..Default::default() },
            ..Default::default()
        })
        .add_plugin(AssetPlugin::default())
        .add_plugin(ScenePlugin::default())
        .add_plugin(WinitPlugin::default())
        .add_plugin(bench::BenchPlugin::default());

    if settings.headless {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        app.add_plugin(MeshPlugin);
    } else {
        app.add_plugin(RenderPlugin::default());
    }

    app.add_plugin(ImagePlugin::default())
        .add_plugin(CorePipelinePlugin::default())
        .add_plugin(PbrPlugin::default());

    match &settings.physics_plugin {
        PhysicsPlugin::Default => {
            app.add_plugin(bevy_rapier3d::plugin::RapierPhysicsPlugin::<
                bevy_rapier3d::plugin::NoUserData,
            >::default());
        }
        PhysicsPlugin::Server { address, compress } => {
            app.add_plugin(physics::RapierPhysicsPlugin {
                address: address.clone(),
                compress: *compress,
            });
        }
    }

    app
        .add_startup_system(ball_collides_with_stack)
        .add_system(bevy_window::close_on_esc)
        .insert_resource(settings);

    app.run()
}

fn ball_collides_with_stack(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<Settings>,
) {
    // Add a camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(
            settings.scene.camera.0,
            settings.scene.camera.1,
            settings.scene.camera.2,
        )
        .looking_at(Vec3::new(0.0, 30.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(150.0, 0.1, 150.0))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Box::new(300.0, 0.2, 300.0).into()),
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

    let (collider, mesh): (Collider, Mesh) = match settings.scene.shape.as_str() {
        "cuboid" => (Collider::cuboid(rad, rad, rad), bevy_render::prelude::shape::Cube::new(rad * 2.0).into()),
        "capsule" => (Collider::capsule_y(rad, rad), bevy_render::prelude::shape::Capsule { radius: rad, depth: rad * 2.0, ..Default::default() }.into()),
        "complex" => {
            let cuboid_rad = rad / 2.0;

            let shapes = vec![
                (
                    Isometry::identity(),
                    SharedShape::cuboid(cuboid_rad, cuboid_rad, cuboid_rad),
                ),
                (
                    Isometry::translation(cuboid_rad, cuboid_rad, 0.0),
                    SharedShape::cuboid(cuboid_rad, cuboid_rad, cuboid_rad),
                ),
                (
                    Isometry::translation(-cuboid_rad, cuboid_rad, 0.0),
                    SharedShape::cuboid(cuboid_rad, cuboid_rad, cuboid_rad),
                ),
            ];

            let collider = SharedShape::compound(shapes);

            (collider.into(), bevy_render::prelude::shape::Box::new(rad, rad, rad).into())
        },
        _ => (Collider::ball(rad), bevy_render::prelude::shape::Icosphere { radius: rad, ..Default::default() }.into()),
    };

    let height = settings.scene.num_object / num / num;
    let density = 0.477;

    for i in 0..num {
        for j in 0usize..height {
            for k in 0..num {
                let x = i as f32 * shift - centerx;
                let y = j as f32 * shift * 2.0 + centery;
                let z = k as f32 * shift - centerz;

                let color = Color::rgb(
                    if i % 2 == 0 { 1.0 } else { 0.0 },
                    if j % 2 == 0 { 1.0 } else { 0.0 },
                    if k % 2 == 0 { 1.0 } else { 0.0 },
                );

                commands
                    .spawn(RigidBody::Dynamic)
                    .insert(collider.clone())
                    .insert(ColliderMassProperties::Density(density))
                    .insert(TransformBundle::from(Transform::from_xyz(x, y, z)))
                    .insert(Shape)
                    .insert(PbrBundle {
                        mesh: meshes.add(mesh.clone()),
                        material: materials.add(color.into()),
                        transform: Transform::from_xyz(x, y, z),
                        ..Default::default()
                    });
            }
        }
    }

    commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(10.0))
        .insert(ColliderMassProperties::Density(0.6))
        .insert(Shape)
        .insert(Velocity::linear(Vec3::new(0.0, 0.0, -100.0)))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Icosphere { radius: 10.0, ..Default::default() }.into()),
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            transform: Transform::from_xyz(0.0, 5.0, 150.0),
            ..Default::default()
        });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1.5,
    });
}
