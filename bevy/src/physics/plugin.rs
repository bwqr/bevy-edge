use std::io::{Write, BufWriter, BufReader};

use bevy_log::info_span;
use bevy_rapier3d::prelude::RapierConfiguration;

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    schedule::{StageLabel, SystemStage, IntoSystemDescriptor},
    system::Resource,
};
use crossbeam::channel::{Sender, Receiver, bounded};

use shared::deflate::{Compressor, Decompressor, CONFIG};
use shared::{request::Request, response::{Response, SyncContext}};
use crate::bench::NetworkLog;

use super::systems;

#[derive(Resource)]
pub struct RequestSender(pub Sender<Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<(Response, u128, NetworkLog, NetworkLog)>);

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
            log::debug!("Plugin thread is started");
            let tcp_stream = std::net::TcpStream::connect(address).unwrap();
            log::debug!("TCP connection is established");

            res_tx.send((Response::SyncContext(SyncContext::default()), 0, NetworkLog::default(), NetworkLog::default())).unwrap();

            while let Ok(req) = {
                let _span = info_span!("request_received_over_channel").entered();
                let req = req_rx.recv();
                log::debug!("request is received from bevy");
                req
            }{
                let mut uplink = NetworkLog::default();
                let mut downlink = NetworkLog::default();

                {
                    let _span = info_span!("request_sent").entered();

                    if let Some(level) = compress {
                        let mut compressor = BufWriter::new(Compressor::new(&tcp_stream, level));
                        bincode::serde::encode_into_std_write(req, &mut compressor, CONFIG).unwrap();
                        compressor.flush().unwrap();

                        let Ok(compressor) = compressor.into_inner() else {
                            panic!("failed to get into inner of compressor");
                        };

                        uplink.raw = compressor.total_in();
                        uplink.compressed = compressor.total_out();
                    } else {
                        bincode::serde::encode_into_std_write(req, &mut BufWriter::new(&tcp_stream), CONFIG).unwrap();
                    }

                    // Force flushing the buffer
                    tcp_stream.set_nodelay(true).unwrap();
                    tcp_stream.set_nodelay(false).unwrap();
                }

                log::debug!("request is sent to physics");

                {
                    let _span = info_span!("response_received").entered();
                    let instant = std::time::Instant::now();

                    let res = if compress.is_some() {
                        let mut decompressor = BufReader::new(Decompressor::new(&tcp_stream));
                        let res = bincode::serde::decode_from_std_read(&mut decompressor, CONFIG);
                        let mut decompressor = decompressor.into_inner();
                        decompressor.finish().unwrap();
                        downlink.raw = decompressor.total_out();
                        downlink.compressed = decompressor.total_in();

                        res
                    } else {
                        bincode::serde::decode_from_std_read(&mut BufReader::new(&tcp_stream), CONFIG)
                    };

                    let ctx = match res {
                        Ok(ctx) => ctx,
                        Err(e) => {
                            panic!("Failed to read data {e:?}");
                        },
                    };

                    let duration = instant.elapsed().as_micros();

                    {
                        let _span = info_span!("response_sent_over_channel").entered();
                        if let Err(e) = res_tx.send((ctx, duration, uplink, downlink)) {
                            log::debug!("Failed to send response {e:?}");
                            break;
                        }
                    }
                }
            }
            log::debug!("Shuting down the Plugin thread");

            if let Some(level) = compress {
                let mut compressor = BufWriter::new(Compressor::new(&tcp_stream, level));
                bincode::serde::encode_into_std_write(Request::Shutdown, &mut compressor, CONFIG).unwrap();
                compressor.flush().unwrap();
            } else {
                bincode::serde::encode_into_std_write(Request::Shutdown, &mut BufWriter::new(&tcp_stream), CONFIG).unwrap();
            }

            log::debug!("Plugin thread is finishing");
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
