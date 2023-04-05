use std::net::SocketAddrV4;

use bevy_ecs::prelude::Entity;
use bevy_rapier3d::{
    prelude::{RapierConfiguration, RapierContext},
    utils,
};
use tracing::{debug, error, info_span};
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use crate::request::Request;

mod request;
mod response;

fn main() {
    debug!("starting physics server");
    let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
    tracing_subscriber::registry().with(chrome_layer).init();

    let srv = std::net::TcpListener::bind("0.0.0.0:4001".parse::<SocketAddrV4>().unwrap()).unwrap();

    while let Err(e) = run_physics_server(&srv) {
        error!("An error is occured, {}", e);
    }
}

fn run_physics_server(srv: &std::net::TcpListener) -> Result<(), String> {
    let (mut stream, _) = srv
        .accept()
        .map_err(|e| format!("could not accept incoming request, {e:?}"))?;

    stream.set_nodelay(true).unwrap();

    debug!("a client is connected to input server");

    let _span = info_span!("client_connected", name = "physics_server").entered();

    let mut context = RapierContext::default();
    let config = RapierConfiguration::default();
    let hooks_instance = ();

    while let Ok(req) = {
        let _span = info_span!("request_received", name = "physics_server").entered();
        bincode::deserialize_from::<_, request::Request>(&stream)
    } {
        match req {
            Request::SyncContext(sync_context) => {
                let response = {
                    let _span = info_span!("processing", name = "physics_server").entered();
                    debug!(
                        "received context rigid {}, collider {}",
                        sync_context.rigid_bodies.len(),
                        sync_context.colliders.len(),
                    );

                    let mut response = response::SyncContext {
                        rigid_body_handles: Vec::new(),
                        collider_handles: Vec::new(),
                        transforms: Vec::new(),
                    };

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
                    bincode::serialize_into(&mut stream, &response::Response::SyncContext(response))
                        .unwrap();
                }
            }
        }
    }

    Ok(())
}
