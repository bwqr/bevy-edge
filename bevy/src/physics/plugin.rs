use std::io::{Write, BufWriter, BufReader};

use bevy_log::info_span;
use bevy_rapier3d::prelude::RapierConfiguration;

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    schedule::{StageLabel, SystemStage, IntoSystemDescriptor},
    system::Resource,
};
use crossbeam::channel::{Sender, Receiver, bounded};

use super::systems;

const CONFIG: bincode::config::Configuration = bincode::config::standard();

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

pub struct RapierPhysicsPlugin {
    pub compress: Option<u32>,
    pub address: String,
}

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        let compress = self.compress;
        let address = self.address.clone();

        std::thread::spawn(move || {
            let mut tcp_stream = std::net::TcpStream::connect(address).unwrap();

            res_tx.send(physics::response::Response::SyncContext(physics::response::SyncContext::default())).unwrap();

            while let Ok(req) = {
                let _span = info_span!("request_received_over_channel").entered();
                req_rx.recv()
            }{
                {
                    let _span = info_span!("request_sent").entered();
                    if let Some(level) = compress {
                        let mut compressor = BufWriter::new(physics::deflate::Compressor::new(&tcp_stream, level));
                        bincode::serde::encode_into_std_write(req, &mut compressor, CONFIG).unwrap();
                        compressor.flush().unwrap();
                    } else {
                        bincode::serde::encode_into_std_write(req, &mut tcp_stream, CONFIG).unwrap();
                    }

                    // Force flushing the buffer
                    tcp_stream.set_nodelay(true).unwrap();
                    tcp_stream.set_nodelay(false).unwrap();
                }

                {
                    let _span = info_span!("response_received").entered();

                    let res = if compress.is_some() {
                        bincode::serde::decode_from_std_read(&mut BufReader::new(physics::deflate::Decompressor::new(&tcp_stream)), CONFIG)
                    } else {
                        bincode::serde::decode_from_std_read(&mut BufReader::new(&tcp_stream), CONFIG)
                    };

                    if let Ok(ctx) = res {
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

        app.add_stage_before(
            CoreStage::First,
            PhysicsStage::Writeback,
            SystemStage::parallel().with_system(systems::writeback_rigid_bodies),
        );
    }
}
