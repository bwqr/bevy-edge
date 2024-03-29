use bench::PluginLog;
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
use bevy_rapier3d::{prelude::{Collider, ColliderMassProperties, RigidBody, Velocity}, rapier::prelude::{Isometry, SharedShape}, render::RapierDebugRenderPlugin};
use bevy_render::{
    mesh::MeshPlugin,
    prelude::{Color, Mesh},
    render_resource::{Shader, ShaderLoader},
    texture::ImagePlugin,
    RenderPlugin,
};
use bevy_scene::ScenePlugin;
use bevy_time::TimePlugin;
use bevy_transform::{prelude::Transform, TransformPlugin};
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

    app.insert_resource(settings.clone());

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
            window: bevy_window::WindowDescriptor {
                present_mode: bevy_window::PresentMode::AutoNoVsync,
                title: "bevy-edge".to_string(),
                ..Default::default()
            },
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

            if !settings.headless {
                app.add_plugin(RapierDebugRenderPlugin::default());
            }

            app.add_system_to_stage(bevy_rapier3d::plugin::PhysicsStages::SyncBackend, sync_physics_time);
        }
        PhysicsPlugin::Server { address, .. } => {
            app.add_plugin(physics::RapierPhysicsPlugin {
                address: address.clone(),
            });
        }
    }

    app
        .add_startup_system(shape_collides_with_stack)
        .add_system(bevy_window::close_on_esc);

    app.run()
}

fn shape_collides_with_stack(
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
        .looking_at(Vec3::new(0.0, 3.0, 0.0), Vec3::Y),
        ..Default::default()
    });

    let length = 15.0;
    let thickness = 1.0;

    /* Create the ground. */
    let walls = if let shared::settings::Room::Closed = settings.scene.room {
        vec![
            ((length * 2.0, thickness, length * 2.0), (0.0, -thickness, 0.0)),
            ((length * 2.0, thickness, length * 2.0), (0.0, length * 10.0 - thickness, 0.0)),
            ((thickness, length * 10.0, length * 2.0), (-length, 0.0, 0.0)),
            ((thickness, length * 10.0, length * 2.0), (length, 0.0, 0.0)),
            ((length, length * 10.0, thickness), (0.0, 0.0, length * 2.0)),
            ((length, length * 10.0, thickness), (0.0, 0.0, -length * 2.0)),
        ]
    } else {
        vec![
            ((length * 2.0, thickness, length * 2.0), (0.0, -thickness, 0.0)),
        ]
    };

    for wall in walls {
        commands
            .spawn(Collider::cuboid(wall.0.0, wall.0.1, wall.0.2))
            .insert(RigidBody::Fixed)
            .insert(PbrBundle {
                mesh: meshes.add(bevy_render::prelude::shape::Box::new(wall.0.0 * 2.0, wall.0.1 * 2.0, wall.0.2 * 2.0).into()),
                material: materials.add(Color::BLACK.into()),
                transform: Transform::from_xyz(wall.1.0, wall.1.1, wall.1.2),
                ..Default::default()
            });
    }

    let num = 10;
    let rad = 0.4;

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

    let shift = rad * 2.0 + 0.005;
    let centerx = shift * (num as f32) / 2.0;
    let centery = shift / 2.0;
    let centerz = shift * (num as f32) / 2.0;

    let height = settings.scene.num_object / num / num;
    let density = 0.477;

    for i in 0..num {
        for j in 0usize..height {
            for k in 0..num {
                let x = i as f32 * shift - centerx;
                let y = j as f32 * shift + centery;
                let z = k as f32 * shift - centerz;

                let color = Color::rgb(
                    (i % 3) as f32 * 0.33,
                    (j % 3) as f32 * 0.33,
                    (k % 3) as f32 * 0.33,
                );

                commands
                    .spawn(RigidBody::Dynamic)
                    .insert(collider.clone())
                    .insert(ColliderMassProperties::Density(density))
                    .insert(bevy_rapier3d::geometry::Restitution { coefficient: settings.scene.restitution, combine_rule: bevy_rapier3d::prelude::CoefficientCombineRule::Max })
                    .insert(bevy_rapier3d::geometry::Friction { coefficient: 0.0, combine_rule: bevy_rapier3d::prelude::CoefficientCombineRule::Max })
                    .insert(Shape)
                    .insert(bevy_rapier3d::dynamics::Ccd { enabled: settings.scene.ccd })
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
        .insert(Collider::ball(1.0))
        .insert(ColliderMassProperties::Density(1.0))
        .insert(Shape)
        .insert(Velocity::linear(Vec3::new(0.0, 0.0, -30.0)))
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Icosphere { radius: 1.0, ..Default::default() }.into()),
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            transform: Transform::from_xyz(0.0, 2.0, length - 2.0),
            ..Default::default()
        });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 2.5,
    });
}

fn sync_physics_time(mut physics_time: ResMut<bevy_rapier3d::plugin::PhysicsTime>, mut log: ResMut<PluginLog>) {
    log.physics_time = physics_time.0;
    physics_time.0 = 0;
}
