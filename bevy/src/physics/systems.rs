use bevy_ecs::{
    prelude::Entity,
    query::{Without, With},
    system::{Commands, Query, Res, ResMut},
};
use bevy_hierarchy::Parent;
use bevy_log::info_span;
use bevy_rapier3d::{
    prelude::{
        ActiveCollisionTypes, ActiveEvents, ActiveHooks, AdditionalMassProperties, Ccd, Collider,
        ColliderDisabled, ColliderMassProperties, CollisionGroups, ContactForceEventThreshold,
        Damping, Dominance, ExternalForce, Friction, GravityScale, LockedAxes,
        RapierColliderHandle, RapierConfiguration, RapierRigidBodyHandle, ReadMassProperties,
        Restitution, RigidBody, RigidBodyDisabled, Sensor, Sleeping, SolverGroups, Velocity,
    },
    rapier::prelude::{ColliderBuilder, RigidBodyBuilder},
    utils,
};
use bevy_time::Time;
use bevy_transform::{prelude::{GlobalTransform, Transform}, TransformBundle};
use shared::{request::{Request, SyncContext}, response::Response};

use crate::bench::PluginLog;

use super::plugin::{RequestSender, ResponseReceiver};

pub type RigidBodyComponents<'a> = (
    Entity,
    &'a RigidBody,
    Option<&'a GlobalTransform>,
    Option<&'a Velocity>,
    Option<&'a AdditionalMassProperties>,
    Option<&'a ReadMassProperties>,
    Option<&'a LockedAxes>,
    Option<&'a ExternalForce>,
    Option<&'a GravityScale>,
    Option<&'a Ccd>,
    Option<&'a Dominance>,
    Option<&'a Sleeping>,
    Option<&'a Damping>,
    Option<&'a RigidBodyDisabled>,
);

pub type ColliderComponents<'a> = (
    Entity,
    &'a Collider,
    Option<&'a Sensor>,
    Option<&'a ColliderMassProperties>,
    Option<&'a ActiveEvents>,
    Option<&'a ActiveHooks>,
    Option<&'a ActiveCollisionTypes>,
    Option<&'a Friction>,
    Option<&'a Restitution>,
    Option<&'a CollisionGroups>,
    Option<&'a SolverGroups>,
    Option<&'a ContactForceEventThreshold>,
    Option<&'a ColliderDisabled>,
);

pub fn init_rigid_bodies(
    context: Res<super::plugin::LocalContext>,
    mut sync_rigid_body: ResMut<super::plugin::RigidBody>,
    rigid_bodies: Query<RigidBodyComponents, Without<RapierRigidBodyHandle>>,
) {
    log::debug!("initting rigid bodies");

    for (
        entity,
        rb,
        transform,
        vel,
        additional_mass_props,
        _mass_props,
        locked_axes,
        force,
        gravity_scale,
        ccd,
        dominance,
        sleep,
        damping,
        disabled,
    ) in rigid_bodies.iter()
    {
        let mut builder = RigidBodyBuilder::new((*rb).into());
        builder = builder.enabled(disabled.is_none());

        if let Some(transform) = transform {
            builder = builder.position(utils::transform_to_iso(
                &transform.compute_transform(),
                context.physics_scale,
            ));
        }

        if let Some(vel) = vel {
            builder = builder
                .linvel((vel.linvel / context.physics_scale).into())
                .angvel(vel.angvel.into());
        }

        if let Some(locked_axes) = locked_axes {
            builder = builder.locked_axes((*locked_axes).into())
        }

        if let Some(gravity_scale) = gravity_scale {
            builder = builder.gravity_scale(gravity_scale.0);
        }

        if let Some(ccd) = ccd {
            builder = builder.ccd_enabled(ccd.enabled)
        }

        if let Some(dominance) = dominance {
            builder = builder.dominance_group(dominance.groups)
        }

        if let Some(sleep) = sleep {
            builder = builder.sleeping(sleep.sleeping);
        }

        if let Some(damping) = damping {
            builder = builder
                .linear_damping(damping.linear_damping)
                .angular_damping(damping.angular_damping);
        }

        if let Some(mprops) = additional_mass_props {
            builder = match mprops {
                AdditionalMassProperties::MassProperties(mprops) => {
                    builder.additional_mass_properties(mprops.into_rapier(context.physics_scale))
                }
                AdditionalMassProperties::Mass(mass) => builder.additional_mass(*mass),
            };
        }

        builder = builder.user_data(entity.to_bits() as u128);

        let mut rb = builder.build();

        #[allow(clippy::useless_conversion)] // Need to convert if dim3 enabled
        if let Some(force) = force {
            rb.add_force((force.force / context.physics_scale).into(), false);
            rb.add_torque(force.torque.into(), false);
        }

        // NOTE: we can’t apply impulses yet at this point because
        //       the rigid-body’s mass isn’t up-to-date yet (its
        //       attached colliders, if any, haven’t been created yet).

        if let Some(sleep) = sleep {
            let activation = rb.activation_mut();
            activation.linear_threshold = sleep.linear_threshold;
            activation.angular_threshold = sleep.angular_threshold;
        }

        builder = builder.user_data(entity.to_bits() as u128);

        sync_rigid_body.0.push(builder.build());
    }
}

pub fn init_colliders(
    config: Res<RapierConfiguration>,
    mut sync_collider: ResMut<super::plugin::Collider>,
    context: Res<super::plugin::LocalContext>,
    colliders: Query<(ColliderComponents, Option<&GlobalTransform>), Without<RapierColliderHandle>>,
    parent_query: Query<(&Parent, Option<&Transform>), With<RapierRigidBodyHandle>>,
) {
    for (
        (
            entity,
            shape,
            sensor,
            mprops,
            active_events,
            active_hooks,
            active_collision_types,
            friction,
            restitution,
            collision_groups,
            solver_groups,
            contact_force_event_threshold,
            disabled,
        ),
        _global_transform,
    ) in colliders.iter()
    {
        let mut scaled_shape = shape.clone();
        scaled_shape.set_scale(
            shape.scale() / context.physics_scale,
            config.scaled_shape_subdivision,
        );
        let mut builder = ColliderBuilder::new(scaled_shape.raw.clone());

        builder = builder.sensor(sensor.is_some());
        builder = builder.enabled(disabled.is_none());

        if let Some(mprops) = mprops {
            builder = match mprops {
                ColliderMassProperties::Density(density) => builder.density(*density),
                ColliderMassProperties::Mass(mass) => builder.mass(*mass),
                ColliderMassProperties::MassProperties(mprops) => {
                    builder.mass_properties(mprops.into_rapier(context.physics_scale))
                }
            };
        }

        if let Some(active_events) = active_events {
            builder = builder.active_events((*active_events).into());
        }

        if let Some(active_hooks) = active_hooks {
            builder = builder.active_hooks((*active_hooks).into());
        }

        if let Some(active_collision_types) = active_collision_types {
            builder = builder.active_collision_types((*active_collision_types).into());
        }

        if let Some(friction) = friction {
            builder = builder
                .friction(friction.coefficient)
                .friction_combine_rule(friction.combine_rule.into());
        }

        if let Some(restitution) = restitution {
            builder = builder
                .restitution(restitution.coefficient)
                .restitution_combine_rule(restitution.combine_rule.into());
        }

        if let Some(collision_groups) = collision_groups {
            builder = builder.collision_groups((*collision_groups).into());
        }

        if let Some(solver_groups) = solver_groups {
            builder = builder.solver_groups((*solver_groups).into());
        }

        if let Some(threshold) = contact_force_event_threshold {
            builder = builder.contact_force_event_threshold(threshold.0);
        }

        builder = builder.user_data(entity.to_bits() as u128);

        let mut body_entity = entity;
        let mut child_transform = Transform::default();
        while let Ok(parent) = parent_query.get(body_entity) {
            if let Some(transform) = parent.1 {
                child_transform = *transform * child_transform;
            }
            body_entity = parent.0.get();
        }

        builder = builder.user_data(entity.to_bits() as u128);

        builder = builder.position(utils::transform_to_iso(&child_transform, context.physics_scale));

        sync_collider.0.push(builder);
    }
}

pub fn send_context(
    time: Res<Time>,
    mut rigid_bodies: ResMut<super::plugin::RigidBody>,
    mut colliders: ResMut<super::plugin::Collider>,
    request: Res<RequestSender>,
) {
    log::debug!("sending context");

    request
        .0
        .send(Request::SyncContext(SyncContext {
            rigid_bodies: std::mem::replace(&mut rigid_bodies.0, Vec::new()),
            colliders: std::mem::replace(&mut colliders.0, Vec::new()),
            delta_seconds: time.delta_seconds(),
        }))
        .unwrap();
}

pub fn writeback_rigid_bodies(mut commands: Commands, response: Res<ResponseReceiver>, mut log: ResMut<PluginLog>) {
    log::debug!("writing back");

    let _span = info_span!("writeback", name = "physics").entered();
    match response.0.recv().unwrap() {
        (Response::SyncContext(sync_context), plugin_log) => {
            *log = plugin_log;

            let _span = info_span!("response_received", name = "physics").entered();

            for (entity, handle) in sync_context.rigid_body_handles {
                commands
                    .entity(Entity::from_bits(entity))
                    .insert(RapierRigidBodyHandle(handle));
            }

            for (entity, handle) in sync_context.collider_handles {
                commands
                    .entity(Entity::from_bits(entity))
                    .insert(RapierColliderHandle(handle));
            }

            for (entity, transform) in sync_context.transforms {
                commands
                    .entity(Entity::from_bits(entity))
                    .insert(TransformBundle::from(transform));
            }
        }
    }
}
