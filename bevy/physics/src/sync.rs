use bevy_ecs::system::Resource;

#[derive(Resource)]
pub struct RigidBody(pub Vec<bevy_rapier3d::rapier::dynamics::RigidBody>);

#[derive(Resource)]
pub struct Collider(pub Vec<bevy_rapier3d::rapier::prelude::ColliderBuilder>);
