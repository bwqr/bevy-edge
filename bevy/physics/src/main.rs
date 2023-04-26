use std::{net::SocketAddrV4, io::{Write, BufWriter, BufReader}};

use bevy_ecs::prelude::Entity;
use bevy_rapier3d::{
    prelude::{RapierConfiguration, RapierContext},
    utils,
};
use tracing::{debug, error, info_span};
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use shared::settings::Settings;
use crate::request::Request;

mod deflate;
mod request;
mod response;

const CONFIG: bincode::config::Configuration = bincode::config::standard();

fn main() {
    debug!("starting physics server");

    let settings_path = std::env::args()
        .collect::<Vec<String>>()
        .get(1)
        .map(|s| s.to_owned())
        .unwrap_or("Settings.ron".to_string());

    let settings: Settings = ron::de::from_reader(std::fs::File::open(settings_path).unwrap()).unwrap();
    let shared::settings::PhysicsPlugin::Server { compress, .. } = settings.physics_plugin else {
        panic!("We cannot run server while settings set to default");
    };

    if let Some(_) = settings.tracing_level {
        let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
    }

    let srv = std::net::TcpListener::bind("0.0.0.0:4001".parse::<SocketAddrV4>().unwrap()).unwrap();

    while let Err(e) = run_physics_server(&srv, compress) {
        error!("An error is occured, {}", e);
    }
}

fn run_physics_server(srv: &std::net::TcpListener, compress: Option<u32>) -> Result<(), String> {
    let (mut tcp_stream, _) = srv
        .accept()
        .map_err(|e| format!("could not accept incoming request, {e:?}"))?;
    println!("accepted client");

    let _span = info_span!("client_connected", name = "physics_server").entered();

    let mut context = RapierContext::default();
    let config = RapierConfiguration::default();
    let hooks_instance = ();

    while let Ok(req) = {
        let _span = info_span!("request_received", name = "physics_server").entered();
        let req = if compress.is_some() {
            bincode::serde::decode_from_std_read::<request::Request, _, _>(&mut BufReader::new(deflate::Decompressor::new(&tcp_stream)), CONFIG)
        } else {
            bincode::serde::decode_from_std_read::<request::Request, _, _>(&mut BufReader::new(&tcp_stream), CONFIG)
        };
        println!("received request");
        req
    } {
        match req {
            Request::SyncContext(sync_context) => {
                let response = {
                    let _span = info_span!("processing", name = "physics_server").entered();

                    let mut response = response::SyncContext::default();

                    for rb in sync_context.rigid_bodies {
                        let entity = Entity::from_bits(rb.user_data as u64);
                        let handle = context.bodies.insert(rb);

                        context.entity2body.insert(entity, handle);
                        //context.last_body_transform_set.insert(handle, *rb.);
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

                    response
                };

                {
                    let _span = info_span!("responded", name = "physics_server").entered();

                    println!("Number of objects {}", response.transforms.len());

                    if let Some(level) = compress {
                        let mut compressor = BufWriter::new(physics::deflate::Compressor::new(&tcp_stream, level));
                        bincode::serde::encode_into_std_write(&response::Response::SyncContext(response), &mut compressor, CONFIG)
                            .unwrap();
                        compressor.flush().unwrap();
                    } else {
                        bincode::serde::encode_into_std_write(&response::Response::SyncContext(response), &mut tcp_stream, CONFIG)
                            .unwrap();
                    }

                    // Force flushing the buffer
                    tcp_stream.set_nodelay(true).unwrap();
                    tcp_stream.set_nodelay(false).unwrap();
                }
            }
        }
    }

    Ok(())
}
