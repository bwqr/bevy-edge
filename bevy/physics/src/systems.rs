use bevy_ecs::{
    prelude::Entity,
    query::Without,
    system::{Query, Res, Commands},
};
use bevy_rapier3d::prelude::{RapierRigidBodyHandle, RigidBody};
use bevy_transform::prelude::GlobalTransform;

use crate::plugin::{CreatedBody, LocalContext, RequestSender, Request, ResponseReceiver, Response};

pub type RigidBodyComponents<'a> = (Entity, &'a RigidBody, Option<&'a GlobalTransform>);

pub fn init_rigid_bodies(
    mut commands: Commands,
    context: Res<LocalContext>,
    sender: Res<RequestSender>,
    receiver: Res<ResponseReceiver>,
    rigid_bodies: Query<RigidBodyComponents, Without<RapierRigidBodyHandle>>,
) {
    let mut created_bodies = Vec::<CreatedBody>::new();

    let physics_scale = context.physics_scale;

    for (entity, rb, transform) in rigid_bodies.iter() {
        log::info!("finding");

        if let Some(transform) = transform {
            created_bodies.push(CreatedBody {
                id: entity.to_bits(),
                body: rb.clone(),
                transform: bevy_rapier3d::utils::transform_to_iso(
                    &transform.compute_transform(),
                    physics_scale,
                ),
            });
        }
    }

    sender.0.send(Request::CreateBody(created_bodies)).unwrap();
    let resp = receiver.0.recv().unwrap();

    match resp {
        Response::RigidBodyHandles(handles) => {
            log::info!("received handles, {}", handles.len());

            for handle in handles {
                commands
                    .entity(Entity::from_bits(handle.0))
                    .insert(RapierRigidBodyHandle(handle.1));
            }
        },
        _ => {},
    };
}

pub fn writeback(receiver: Res<ResponseReceiver>) {
    if let Ok(resp) = receiver.0.recv() {
        match resp {
            Response::SimulationResult(result) => { },
            _ => {},
        }
    }
}
