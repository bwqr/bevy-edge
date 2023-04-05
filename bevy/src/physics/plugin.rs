use bevy_log::info_span;
use bevy_rapier3d::prelude::RapierConfiguration;

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    schedule::{StageLabel, SystemStage, IntoSystemDescriptor},
    system::Resource,
};
use crossbeam::channel::{Sender, Receiver, bounded};

use super::systems;

#[derive(Resource)]
pub struct RequestSender(pub Sender<physics::request::Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<physics::response::Response>);

#[derive(Resource)]
pub struct RigidBody(pub Vec<bevy_rapier3d::rapier::dynamics::RigidBody>);

#[derive(Resource)]
pub struct Collider(pub Vec<bevy_rapier3d::rapier::prelude::ColliderBuilder>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    Writeback,
}

#[derive(Resource)]
pub struct LocalContext {
    pub physics_scale: f32,
}

pub struct RapierPhysicsPlugin;

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        std::thread::spawn(move || {
            let mut stream = std::net::TcpStream::connect("192.168.1.240:4001").unwrap();
            stream.set_nodelay(true).unwrap();

            while let Ok(req) = {
                let _span = info_span!("request_received_over_channel").entered();
                req_rx.recv()
            }{
                {
                    let _span = info_span!("request_sent").entered();
                    bincode::serialize_into(&mut stream, &req).unwrap();
                }

                {
                    let _span = info_span!("response_received").entered();

                    if let Ok(ctx) = bincode::deserialize_from(&stream) {
                        let _span = info_span!("response_sent_over_channel").entered();
                        res_tx.send(ctx).unwrap();
                    }
                }
            }
        });

        if app.world.get_resource::<RapierConfiguration>().is_none() {
            app.insert_resource(RapierConfiguration::default());
        }

        app.insert_resource(RequestSender(req_tx));
        app.insert_resource(ResponseReceiver(res_rx));
        app.insert_resource(LocalContext { physics_scale: 1.0 });

        app.insert_resource(RigidBody(Vec::new()));
        app.insert_resource(Collider(Vec::new()));

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
