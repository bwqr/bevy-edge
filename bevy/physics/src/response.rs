use bevy_rapier3d::rapier::prelude::{RigidBodyHandle, ColliderHandle};
use bevy_transform::prelude::Transform;

pub struct SyncContext {
    pub rigid_body_handles: Vec<(u64, RigidBodyHandle)>,
    pub collider_handles: Vec<(u64, ColliderHandle)>,
    pub transforms: Vec<(u64, Transform)>,
}

pub enum Response {
    SyncContext(SyncContext)
}
