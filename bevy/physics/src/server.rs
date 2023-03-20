use bevy_ecs::prelude::Entity;
use bevy_rapier3d::{
    prelude::{RapierConfiguration, RapierContext},
    utils,
};
use crossbeam::channel::{Receiver, Sender};

use crate::{request::Request, response};

pub fn start_physics_server(
    request_rx: Receiver<crate::request::Request>,
    response_tx: Sender<crate::response::Response>,
) {
    std::thread::spawn(move || {
        log::debug!("starting physics server");

        let mut context = RapierContext::default();
        let config = RapierConfiguration::default();
        let hooks_instance = ();

        while let Ok(req) = request_rx.recv() {
            match req {
                Request::SyncContext(sync_context) => {
                    log::debug!(
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
                            context.colliders.insert_with_parent(collider, *body_handle, &mut context.bodies)
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
                        &sync_context.time,
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

                    response_tx
                        .send(response::Response::SyncContext(response))
                        .unwrap();
                }
            }
        }
    });
}
