use bevy_app::App;
use bevy_asset::{AssetPlugin, Assets};
use bevy_core::CorePlugin;
use bevy_core_pipeline::{prelude::Camera3dBundle, CorePipelinePlugin};
use bevy_ecs::{
    query::With,
    system::{Commands, Query, ResMut, Res}, prelude::Component,
};
use bevy_input::InputPlugin;
use bevy_log::LogPlugin;
use bevy_math::Vec3;
use bevy_pbr::{PbrBundle, PbrPlugin, PointLight, PointLightBundle, StandardMaterial};
use bevy_rapier3d::prelude::{Collider, Restitution, RigidBody};
use bevy_render::{
    prelude::{Color, Mesh},
    texture::ImagePlugin,
    RenderPlugin,
};
use bevy_scene::ScenePlugin;
use bevy_time::{TimePlugin, Time};
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

    app
        .add_plugin(LogPlugin::default())
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
        .add_plugin(plugin::RapierPhysicsPlugin)
        //.add_plugin(bevy_rapier3d::plugin::RapierPhysicsPlugin::<bevy_rapier3d::plugin::NoUserData>::default())
        //.add_plugin(bevy_rapier3d::render::RapierDebugRenderPlugin::default())
        ;

    app
        .add_startup_system(setup_graphics)
        .add_startup_system(setup_physics)
        .add_system(rotate)
        .add_system(print_ball_altitude)
        .add_system(bevy_window::close_on_esc);

    app.run()
}

fn setup_graphics(mut commands: Commands) {
    // Add a camera so we can see the debug-render.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-3.0, 3.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn setup_physics(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    /* Create the ground. */
    commands
        .spawn(Collider::cuboid(100.0, 0.1, 100.0))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, -2.0, 0.0)));

    /* Create the bouncing ball. */
    commands
        .spawn(RigidBody::Dynamic)
        .insert(Collider::ball(0.5))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 4.0, 0.0)))
        .insert(Shape)
        .insert(PbrBundle {
            mesh: meshes.add(bevy_render::prelude::shape::Cube::default().into()),
            material: materials.add(Color::YELLOW.into()),
            transform: Transform::from_xyz(0.0, 4.0, 0.0),
            ..Default::default()
        });

    // Add a light source for better 3d visibility.
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 100.,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::ONE * 3.0),
        ..Default::default()
    });
}
fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

fn print_ball_altitude(positions: Query<&Transform, With<RigidBody>>) {
    for transform in positions.iter() {
        log::debug!("Ball altitude: {}", transform.translation.y);
    }
}
