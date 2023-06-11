use bevy_rapier3d::rapier::prelude::{ColliderHandle, RigidBodyHandle};
use bevy_transform::prelude::Transform;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct Log {
    pub physics_time: u32,
    pub compress_time: u32,
    pub decompress_time: u32,
}

#[derive(Default, Deserialize, Serialize)]
pub struct SyncContext {
    pub rigid_body_handles: Vec<(u64, RigidBodyHandle)>,
    pub collider_handles: Vec<(u64, ColliderHandle)>,
    pub transforms: Vec<(u64, Transform)>,
}

#[derive(Deserialize, Serialize)]
pub enum Response {
    SyncContext(SyncContext),
}
