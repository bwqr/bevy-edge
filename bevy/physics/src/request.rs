use bevy_rapier3d::rapier::{dynamics::RigidBody, prelude::ColliderBuilder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SyncContext {
    pub rigid_bodies: Vec<RigidBody>,
    pub colliders: Vec<ColliderBuilder>,
}

#[derive(Deserialize, Serialize)]
pub enum Request {
    SyncContext(SyncContext),
}
