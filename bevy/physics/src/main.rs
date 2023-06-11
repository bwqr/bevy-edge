use std::{net::SocketAddrV4, io::{Write, BufWriter, BufReader}};

use bevy_ecs::prelude::Entity;
use bevy_rapier3d::{
    prelude::{RapierConfiguration, RapierContext},
    utils,
};
use log::debug;
use tracing::info_span;
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use shared::{deflate::{Compressor, CONFIG, Decompressor}, settings::Settings};
use shared::{request::Request, response::{Response, SyncContext, Log}};

fn main() {
    env_logger::init();

    debug!("starting physics server");

    let srv = std::net::TcpListener::bind("0.0.0.0:4001".parse::<SocketAddrV4>().unwrap()).unwrap();

    run_physics_server(&srv);

    debug!("Server is finished, terminating");
}

fn run_physics_server(srv: &std::net::TcpListener) {
    let (tcp_stream, _) = srv
        .accept()
        .unwrap();

    let settings: Settings = bincode::serde::decode_from_std_read(&mut &tcp_stream, CONFIG).unwrap();

    println!("{}", settings);

    let compress = match settings.physics_plugin {
        shared::settings::PhysicsPlugin::Server { compress, .. } => compress,
        shared::settings::PhysicsPlugin::Default => None,
    };

    if let Some(_) = settings.tracing_level {
        let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
    }

    let _span = info_span!("client_connected", name = "physics_server").entered();

    debug!("accepted client");

    let mut context = RapierContext::default();
    let config = RapierConfiguration::default();
    let hooks_instance = ();
    let mut frame_count = 0;

    loop {
        let mut log = Log::default();

        let req = if compress.is_some() {
            let _span = info_span!("request_received", name = "physics_server").entered();
            let mut decompressor = BufReader::new(Decompressor::new(&tcp_stream));
            let req = bincode::serde::decode_from_std_read::<Request, _, _>(&mut decompressor, CONFIG).unwrap();
            let mut decompressor = decompressor.into_inner();
            decompressor.finish().unwrap();
            log.decompress_time = decompressor.elapsed();

            req
        } else {
            let _span = info_span!("request_received", name = "physics_server").entered();
            bincode::serde::decode_from_std_read::<Request, _, _>(&mut BufReader::new(&tcp_stream), CONFIG).unwrap()
        };

        match req {
            Request::Shutdown => {
                log::debug!("shutdown is received");
                return;
            },
            Request::SyncContext(sync_context) => {
                let response = {
                    let _span = info_span!("processing", name = "physics_server").entered();

                    let instant = std::time::Instant::now();

                    let mut response = SyncContext::default();

                    for rb in sync_context.rigid_bodies {
                        let entity = Entity::from_bits(rb.user_data as u64);
                        let handle = context.bodies.insert(rb);

                        context.entity2body.insert(entity, handle);
                        //context.last_body_transform_set.insert(handle, bevy_transform::prelude::GlobalTransform::IDENTITY);
                        response.rigid_body_handles.push((entity.to_bits(), handle));
                    }

                    for collider in sync_context.colliders {
                        let entity = Entity::from_bits(collider.user_data as u64);
                        let handle = if let Some(body_handle) = context.entity2body.get(&entity) {
                            context.colliders.insert_with_parent(
                                collider,
                                *body_handle,
                                &mut context.bodies,
                            )
                        } else {
                            context.colliders.insert(collider)
                        };

                        context.entity2collider.insert(entity, handle);
                        response.collider_handles.push((entity.to_bits(), handle));
                    }

                    context.step_simulation(
                        config.gravity,
                        config.timestep_mode,
                        None,
                        &hooks_instance,
                        sync_context.delta_seconds,
                        &mut bevy_rapier3d::prelude::SimulationToRenderTime { diff: 0.0 },
                        None,
                    );

                    for (_, rb) in context.bodies.iter() {
                        let interpolated_pos =
                            utils::iso_to_transform(rb.position(), context.physics_scale());
                        response
                            .transforms
                            .push((rb.user_data as u64, interpolated_pos));
                    }

                    log.physics_time = instant.elapsed().as_micros().try_into().unwrap();

                    response
                };

                {
                    let _span = info_span!("responded", name = "physics_server").entered();

                    if let Some(level) = compress {
                        let mut compressor = BufWriter::new(Compressor::new(&tcp_stream, level));
                        bincode::serde::encode_into_std_write(&Response::SyncContext(response), &mut compressor, CONFIG)
                            .unwrap();
                        compressor.flush().unwrap();
                        let compressor = compressor.into_inner().map_err(|_| "failed to get inner of compressor buffer writer").unwrap();
                        log.compress_time = compressor.elapsed();
                    } else {
                        bincode::serde::encode_into_std_write(&Response::SyncContext(response), &mut BufWriter::new(&tcp_stream), CONFIG)
                            .unwrap();
                    }

                    {
                        bincode::serde::encode_into_std_write(&log, &mut BufWriter::with_capacity(1024, &tcp_stream), CONFIG).unwrap();
                    }

                    // Force flushing the buffer
                    tcp_stream.set_nodelay(true).unwrap();
                    tcp_stream.set_nodelay(false).unwrap();
                }
            }
        }

        frame_count += 1;
        log::debug!("frame {}", frame_count);
    }
}
