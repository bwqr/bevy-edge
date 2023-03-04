use std::{collections::HashMap, ops::{Deref, DerefMut}};

use bevy_app::{App, Plugin};
use bevy_ecs::{
    query::{Changed, Without},
    schedule::{Stage, StageLabel, SystemStage, IntoSystemDescriptor},
    system::{Query, Res, ResMut, Resource, Commands},
    world::World,
};
use bevy_rapier3d::{
    prelude::{
        NoUserData, RapierConfiguration, RapierContext, RapierRigidBodyHandle, RigidBody,
        SimulationToRenderTime, TransformInterpolation, systems::{RigidBodyComponents, RigidBodyWritebackComponents, ColliderComponents}, AdditionalMassProperties, TimestepMode, Velocity, RapierColliderHandle, ReadMassProperties, ColliderMassProperties, MassProperties,
    },
    rapier::prelude::{RigidBodyHandle, RigidBodyType, RigidBodyBuilder, ColliderBuilder},
};
use bevy_time::Time;
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_hierarchy::prelude::Parent;

#[derive(Resource, Default)]
struct ScratchRapier(RapierContext, RapierConfiguration);

#[derive(Resource, Default)]
pub(crate) struct MainWorld(pub World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    StepSimulation,
    Writeback
}

pub struct RapierPhysicsPlugin;

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app
            .init_resource::<ScratchRapier>();

        let mut physics_app = App::new();

        physics_app
            .add_stage(
                PhysicsStage::SyncBackend,
                SystemStage::parallel()
                    .with_system(apply_rigid_body_user_changes)
                    .with_system(init_rigid_bodies.after(apply_rigid_body_user_changes))
                    .with_system(
                        init_colliders
                            .after(init_rigid_bodies)
                    )
            )
            .add_stage(
                PhysicsStage::StepSimulation,
                SystemStage::parallel().with_system(step_simulation),
            )
            .add_stage(
                PhysicsStage::Writeback,
                SystemStage::parallel().with_system(writeback_rigid_bodies),
            );

        physics_app.insert_resource(
            app.world
                .remove_resource::<RapierConfiguration>()
                .unwrap_or_else(|| RapierConfiguration::default()),
        );

        physics_app
            .insert_resource(SimulationToRenderTime::default())
            .insert_resource(RapierContext::default())
            .insert_resource(Time::default());

        app.add_sub_app("PhysicsApp", physics_app, |app_world, physics_app| {
            physics_app.world.insert_resource(app_world.resource::<Time>().clone());

            run_in_app_world(app_world, physics_app, PhysicsStage::SyncBackend);

            //run_in_app_world(app_world, physics_app, PhysicsStage::StepSimulation);
            {
                let step_simulation = physics_app
                    .schedule
                    .get_stage_mut::<SystemStage>(PhysicsStage::StepSimulation)
                    .unwrap();

                step_simulation.run(&mut physics_app.world);
            }

            run_in_app_world(app_world, physics_app, PhysicsStage::Writeback);
        });
    }
}

fn run_in_app_world(app_world: &mut World, physics_app: &mut App, stage: PhysicsStage) {
    log::debug!("running stage for physics app in app world");

    let stage = physics_app
        .schedule
        .get_stage_mut::<SystemStage>(stage)
        .unwrap();

    let ctx = physics_app.world.remove_resource::<RapierContext>().unwrap();
    let conf = physics_app.world.remove_resource::<RapierConfiguration>().unwrap();
    let sim_time = physics_app.world.remove_resource::<SimulationToRenderTime>().unwrap();

    app_world.insert_resource(ctx);
    app_world.insert_resource(conf);
    app_world.insert_resource(sim_time);

    stage.run(app_world);

    let ctx = app_world.remove_resource::<RapierContext>().unwrap();
    let conf = app_world.remove_resource::<RapierConfiguration>().unwrap();
    let sim_time = app_world.remove_resource::<SimulationToRenderTime>().unwrap();

    stage.apply_buffers(app_world);

    physics_app
        .insert_resource(ctx)
        .insert_resource(conf)
        .insert_resource(sim_time);
}

/// System responsible for applying changes the user made to a rigid-body-related component.
pub fn apply_rigid_body_user_changes(
    mut context: ResMut<RapierContext>,
    config: Res<RapierConfiguration>,
    changed_rb_types: Query<(&RapierRigidBodyHandle, &RigidBody), Changed<RigidBody>>,
    mut changed_transforms: Query<
        (
            &RapierRigidBodyHandle,
            &GlobalTransform,
            Option<&mut TransformInterpolation>,
        ),
        Changed<GlobalTransform>,
    >,
) {
    let context = &mut *context;
    let scale = context.physics_scale();

    for (handle, rb_type) in changed_rb_types.iter() {
        if let Some(rb) = context.bodies.get_mut(handle.0) {
            rb.set_body_type((*rb_type).into(), true);
        }
    }

    let transform_changed =
        |handle: &RigidBodyHandle,
         transform: &GlobalTransform,
         last_transform_set: &HashMap<RigidBodyHandle, GlobalTransform>| {
            if config.force_update_from_transform_changes {
                true
            } else if let Some(prev) = last_transform_set.get(handle) {
                *prev != *transform
            } else {
                true
            }
        };

    for (handle, global_transform, mut interpolation) in changed_transforms.iter_mut() {
        if let Some(interpolation) = interpolation.as_deref_mut() {
            // Reset the interpolation so we don’t overwrite
            // the user’s input.
            interpolation.start = None;
            interpolation.end = None;
        }

        if let Some(rb) = context.bodies.get_mut(handle.0) {
            match rb.body_type() {
                RigidBodyType::KinematicPositionBased => {
                    if transform_changed(
                        &handle.0,
                        global_transform,
                        &context.last_body_transform_set,
                    ) {
                        rb.set_next_kinematic_position(bevy_rapier3d::utils::transform_to_iso(
                            &global_transform.compute_transform(),
                            scale,
                        ));
                        context
                            .last_body_transform_set
                            .insert(handle.0, *global_transform);
                    }
                }
                _ => {
                    if transform_changed(
                        &handle.0,
                        global_transform,
                        &context.last_body_transform_set,
                    ) {
                        rb.set_position(
                            bevy_rapier3d::utils::transform_to_iso(&global_transform.compute_transform(), scale),
                            true,
                        );
                        context
                            .last_body_transform_set
                            .insert(handle.0, *global_transform);
                    }
                }
            }
        }
    }
}

/// System responsible for creating new Rapier rigid-bodies from the related `bevy_rapier` components.
pub fn init_rigid_bodies(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,
    rigid_bodies: Query<RigidBodyComponents, Without<RapierRigidBodyHandle>>,
) {
    let physics_scale = context.physics_scale();

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
        log::info!("adding a new rigid body");

        let mut builder = RigidBodyBuilder::new((*rb).into());
        builder = builder.enabled(disabled.is_none());

        if let Some(transform) = transform {
            builder = builder.position(bevy_rapier3d::utils::transform_to_iso(
                &transform.compute_transform(),
                physics_scale,
            ));
        }

        #[allow(clippy::useless_conversion)] // Need to convert if dim3 enabled
        if let Some(vel) = vel {
            builder = builder
                .linvel((vel.linvel / physics_scale).into())
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
                    builder.additional_mass_properties(mprops.into_rapier(physics_scale))
                }
                AdditionalMassProperties::Mass(mass) => builder.additional_mass(*mass),
            };
        }

        builder = builder.user_data(entity.to_bits() as u128);

        let mut rb = builder.build();

        #[allow(clippy::useless_conversion)] // Need to convert if dim3 enabled
        if let Some(force) = force {
            rb.add_force((force.force / physics_scale).into(), false);
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

        let handle = context.bodies.insert(rb);
        commands
            .entity(entity)
            .insert(RapierRigidBodyHandle(handle));
        context.entity2body.insert(entity, handle);

        if let Some(transform) = transform {
            context.last_body_transform_set.insert(handle, *transform);
        }
    }
}

/// System responsible for creating new Rapier colliders from the related `bevy_rapier` components.
pub fn init_colliders(
    mut commands: Commands,
    config: Res<RapierConfiguration>,
    mut context: ResMut<RapierContext>,
    colliders: Query<(ColliderComponents, Option<&GlobalTransform>), Without<RapierColliderHandle>>,
    mut rigid_body_mprops: Query<&mut ReadMassProperties>,
    parent_query: Query<(&Parent, Option<&Transform>)>,
) {
    let context = &mut *context;
    let physics_scale = context.physics_scale();

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
        global_transform,
    ) in colliders.iter()
    {
        let mut scaled_shape = shape.clone();
        scaled_shape.set_scale(shape.scale() / physics_scale, config.scaled_shape_subdivision);
        let mut builder = ColliderBuilder::new(scaled_shape.raw.clone());

        builder = builder.sensor(sensor.is_some());
        builder = builder.enabled(disabled.is_none());

        if let Some(mprops) = mprops {
            builder = match mprops {
                ColliderMassProperties::Density(density) => builder.density(*density),
                ColliderMassProperties::Mass(mass) => builder.mass(*mass),
                ColliderMassProperties::MassProperties(mprops) => {
                    builder.mass_properties(mprops.into_rapier(physics_scale))
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

        let mut body_entity = entity;
        let mut body_handle = context.entity2body.get(&body_entity).copied();
        let mut child_transform = Transform::default();
        while body_handle.is_none() {
            if let Ok((parent_entity, transform)) = parent_query.get(body_entity) {
                if let Some(transform) = transform {
                    child_transform = *transform * child_transform;
                }
                body_entity = parent_entity.get();
            } else {
                break;
            }

            body_handle = context.entity2body.get(&body_entity).copied();
        }

        builder = builder.user_data(entity.to_bits() as u128);

        let handle = if let Some(body_handle) = body_handle {
            builder = builder.position(bevy_rapier3d::utils::transform_to_iso(&child_transform, physics_scale));
            let handle =
                context
                    .colliders
                    .insert_with_parent(builder, body_handle, &mut context.bodies);
            if let Ok(mut mprops) = rigid_body_mprops.get_mut(body_entity) {
                // Inserting the collider changed the rigid-body’s mass properties.
                // Read them back from the engine.
                if let Some(parent_body) = context.bodies.get(body_handle) {
                    mprops.0 = MassProperties::from_rapier(
                        parent_body.mass_properties().local_mprops,
                        physics_scale,
                    );
                }
            }
            handle
        } else {
            let global_transform = global_transform.cloned().unwrap_or_default();
            builder = builder.position(bevy_rapier3d::utils::transform_to_iso(
                &global_transform.compute_transform(),
                physics_scale,
            ));
            context.colliders.insert(builder)
        };

        commands.entity(entity).insert(RapierColliderHandle(handle));
        context.entity2collider.insert(entity, handle);
    }
}

fn step_simulation(
    mut context: ResMut<RapierContext>,
    config: Res<RapierConfiguration>,
    (time, mut sim_to_render_time): (Res<Time>, ResMut<SimulationToRenderTime>),
) {
    log::debug!("running StepSimulation stage for physics app");

    let hooks_instance: NoUserData = ();

    context.step_simulation(
        config.gravity,
        config.timestep_mode,
        None,
        &hooks_instance,
        &time,
        &mut sim_to_render_time,
        None,
    );
}

pub fn writeback_rigid_bodies(
    mut context: ResMut<RapierContext>,
    config: Res<RapierConfiguration>,
    sim_to_render_time: Res<SimulationToRenderTime>,
    global_transforms: Query<&GlobalTransform>,
    mut writeback: Query<RigidBodyWritebackComponents>,
) {
    let context = &mut *context;
    let scale = context.physics_scale();

    if config.physics_pipeline_active {
        for (entity, parent, transform, mut interpolation, mut velocity, mut sleeping) in
            writeback.iter_mut()
        {
            // TODO: do this the other way round: iterate through Rapier’s RigidBodySet on the active bodies,
            // and update the components accordingly. That way, we don’t have to iterate through the entities that weren’t changed
            // by physics (for example because they are sleeping).
            if let Some(handle) = context.entity2body.get(&entity).copied() {

                if let Some(rb) = context.bodies.get(handle) {
                    let mut interpolated_pos = bevy_rapier3d::utils::iso_to_transform(rb.position(), scale);

                    if let TimestepMode::Interpolated { dt, .. } = config.timestep_mode {
                        if let Some(interpolation) = interpolation.as_deref_mut() {
                            if interpolation.end.is_none() {
                                interpolation.end = Some(*rb.position());
                            }

                            if let Some(interpolated) =
                                interpolation.lerp_slerp((dt + sim_to_render_time.diff) / dt)
                            {
                                interpolated_pos = bevy_rapier3d::utils::iso_to_transform(&interpolated, scale);
                            }
                        }
                    }

                    if let Some(mut transform) = transform {
                        // NOTE: we query the parent’s global transform here, which is a bit
                        //       unfortunate (performance-wise). An alternative would be to
                        //       deduce the parent’s global transform from the current entity’s
                        //       global transform. However, this makes it nearly impossible
                        //       (because of rounding errors) to predict the exact next value this
                        //       entity’s global transform will get after the next transform
                        //       propagation, which breaks our transform modification detection
                        //       that we do to detect if the user’s transform has to be written
                        //       into the rigid-body.
                        if let Some(parent_global_transform) =
                            parent.and_then(|p| global_transforms.get(**p).ok())
                        {
                            // We need to compute the new local transform such that:
                            // curr_parent_global_transform * new_transform = interpolated_pos
                            // new_transform = curr_parent_global_transform.inverse() * interpolated_pos
                            let (_, inverse_parent_rotation, inverse_parent_translation) =
                                parent_global_transform
                                    .affine()
                                    .inverse()
                                    .to_scale_rotation_translation();
                            let new_rotation = inverse_parent_rotation * interpolated_pos.rotation;

                            #[allow(unused_mut)] // mut is needed in 2D but not in 3D.
                            let mut new_translation = inverse_parent_rotation
                                * interpolated_pos.translation
                                + inverse_parent_translation;

                            if transform.rotation != new_rotation
                                || transform.translation != new_translation
                            {
                                // NOTE: we write the new value only if there was an
                                //       actual change, in order to not trigger bevy’s
                                //       change tracking when the values didn’t change.
                                transform.rotation = new_rotation;
                                transform.translation = new_translation;
                            }

                            // NOTE: we need to compute the result of the next transform propagation
                            //       to make sure that our change detection for transforms is exact
                            //       despite rounding errors.
                            let new_global_transform =
                                parent_global_transform.mul_transform(*transform);

                            context
                                .last_body_transform_set
                                .insert(handle, new_global_transform);
                        } else {
                            if transform.rotation != interpolated_pos.rotation
                                || transform.translation != interpolated_pos.translation
                            {
                                // NOTE: we write the new value only if there was an
                                //       actual change, in order to not trigger bevy’s
                                //       change tracking when the values didn’t change.
                                transform.rotation = interpolated_pos.rotation;
                                transform.translation = interpolated_pos.translation;
                            }

                            context
                                .last_body_transform_set
                                .insert(handle, GlobalTransform::from(interpolated_pos));
                        }
                    }

                    if let Some(velocity) = &mut velocity {
                        let new_vel = Velocity {
                            linvel: (rb.linvel() * scale).into(),
                            angvel: (*rb.angvel()).into(),
                        };

                        // NOTE: we write the new value only if there was an
                        //       actual change, in order to not trigger bevy’s
                        //       change tracking when the values didn’t change.
                        if **velocity != new_vel {
                            **velocity = new_vel;
                        }
                    }

                    if let Some(sleeping) = &mut sleeping {
                        // NOTE: we write the new value only if there was an
                        //       actual change, in order to not trigger bevy’s
                        //       change tracking when the values didn’t change.
                        if sleeping.sleeping != rb.is_sleeping() {
                            sleeping.sleeping = rb.is_sleeping();
                        }
                    }
                }
            }
        }
    }
}
