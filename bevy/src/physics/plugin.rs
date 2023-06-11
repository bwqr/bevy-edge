use std::io::{Write, BufWriter, BufReader, Read};

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
use crate::bench::{PluginLog, NetworkLog, TimeLog};

use super::systems;

struct LogReader<R> {
    reader: R,
    read_bytes: usize,
}

impl<R> LogReader<R> {
    fn new(reader: R) -> Self {
        LogReader { reader, read_bytes: 0 }
    }
}

impl<R: Read> Read for LogReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
            .map(|read_bytes| {
                self.read_bytes += read_bytes;
                read_bytes
            })
    }
}

struct LogWriter<W> {
    writer: W,
    written_bytes: usize,
}

impl<W> LogWriter<W> {
    fn new(writer: W) -> Self {
        LogWriter { writer, written_bytes: 0 }
    }
}

impl<W: Write> Write for LogWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
            .map(|written_bytes| {
                self.written_bytes += written_bytes;
                written_bytes
            })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[derive(Resource)]
pub struct RequestSender(pub Sender<Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<(Response, PluginLog)>);

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
    pub address: String,
}

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        let settings = app.world.get_resource::<shared::settings::Settings>().unwrap().clone();
        let address = self.address.clone();

        std::thread::spawn(move || {
            log::debug!("Plugin thread is started");
            let tcp_stream = std::net::TcpStream::connect(address).unwrap();
            log::debug!("TCP connection is established");

            bincode::serde::encode_into_std_write(&settings, &mut &tcp_stream, CONFIG).unwrap();

            let compress = match settings.physics_plugin {
                shared::settings::PhysicsPlugin::Server { compress, .. } => compress,
                shared::settings::PhysicsPlugin::Default => None,
            };

            res_tx.send((Response::SyncContext(SyncContext::default()), PluginLog::default())).unwrap();

            while let Ok(req) = {
                let _span = info_span!("request_received_over_channel").entered();
                let req = req_rx.recv();
                log::debug!("request is received from bevy");
                req
            }{
                let mut uplink = NetworkLog::default();
                let mut downlink = NetworkLog::default();
                let mut comp_time = TimeLog::default();

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
                        comp_time.compress = compressor.elapsed();
                    } else {
                        let mut writer = BufWriter::new(LogWriter::new(&tcp_stream));
                        bincode::serde::encode_into_std_write(req, &mut writer, CONFIG).unwrap();
                        writer.flush().unwrap();
                        uplink.raw = writer.into_inner().map_err(|_| "failed to get inner of buffer writer").unwrap().written_bytes.try_into().unwrap();
                    }

                    // Force flushing the buffer
                    tcp_stream.set_nodelay(true).unwrap();
                    tcp_stream.set_nodelay(false).unwrap();
                }

                log::debug!("request is sent to physics");

                {
                    let _span = info_span!("response_received").entered();
                    let instant = std::time::Instant::now();

                    let log: shared::response::Log;

                    let ctx = if compress.is_some() {
                        let mut decompressor = BufReader::new(Decompressor::new(&tcp_stream));
                        let res = bincode::serde::decode_from_std_read(&mut decompressor, CONFIG).unwrap();
                        let mut decompressor = decompressor.into_inner();
                        decompressor.finish().unwrap();
                        downlink.raw = decompressor.total_out();
                        downlink.compressed = decompressor.total_in();
                        comp_time.decompress = decompressor.elapsed();
                        log = bincode::serde::decode_from_std_read(&mut BufReader::with_capacity(1024, &tcp_stream), CONFIG).unwrap();

                        res
                    } else {
                        let mut reader = BufReader::new(LogReader::new(&tcp_stream));
                        let res = bincode::serde::decode_from_std_read(&mut reader, CONFIG).unwrap();
                        log = bincode::serde::decode_from_std_read(&mut reader, CONFIG).unwrap();
                        downlink.raw = reader.into_inner().read_bytes.try_into().unwrap();

                        res
                    };

                    let plugin_log = PluginLog {
                        physics_time: log.physics_time,
                        network_time: instant.elapsed().as_micros().try_into().unwrap(),
                        uplink,
                        downlink,
                        client: comp_time,
                        server: TimeLog { compress: log.compress_time, decompress: log.decompress_time }
                    };

                    if let Err(e) = res_tx.send((ctx, plugin_log)) {
                        log::debug!("Failed to send response {e:?}");
                        break;
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
