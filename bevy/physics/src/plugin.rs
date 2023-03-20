use bevy_rapier3d::prelude::RapierConfiguration;
use crossbeam::channel::{bounded, Receiver, Sender};

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    schedule::{IntoSystemDescriptor, StageLabel, SystemStage},
    system::Resource,
};

use crate::systems;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    Writeback,
}

#[derive(Resource)]
pub struct RequestSender(pub Sender<crate::request::Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<crate::response::Response>);
#[derive(Resource)]
pub struct LocalContext {
    pub physics_scale: f32,
}

pub struct RapierPhysicsPlugin;

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        crate::server::start_physics_server(req_rx.clone(), res_tx.clone());

        if app.world.get_resource::<RapierConfiguration>().is_none() {
            app.insert_resource(RapierConfiguration::default());
        }

        app.insert_resource(RequestSender(req_tx));
        app.insert_resource(ResponseReceiver(res_rx));
        app.insert_resource(LocalContext { physics_scale: 1.0 });

        app.insert_resource(crate::sync::RigidBody(Vec::new()));
        app.insert_resource(crate::sync::Collider(Vec::new()));

        app.add_stage_after(
            CoreStage::Update,
            PhysicsStage::SyncBackend,
            SystemStage::parallel()
                .with_system(systems::init_rigid_bodies)
                .with_system(systems::init_colliders.after(systems::init_rigid_bodies))
                .with_system(systems::send_context.after(systems::init_colliders)),
        );

        app.add_stage_after(
            PhysicsStage::SyncBackend,
            PhysicsStage::Writeback,
            SystemStage::parallel().with_system(systems::writeback_rigid_bodies),
        );
    }
}
