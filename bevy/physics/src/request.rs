use bevy_rapier3d::rapier::{dynamics::RigidBody, prelude::ColliderBuilder};
use bevy_time::Time;

pub struct SyncContext {
    pub rigid_bodies: Vec<RigidBody>,
    pub colliders: Vec<ColliderBuilder>,
    pub time: Time,
}

pub enum Request {
    SyncContext(SyncContext),
}
