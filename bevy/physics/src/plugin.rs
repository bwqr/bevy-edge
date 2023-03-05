use crossbeam::channel::{Receiver, bounded, Sender};

use bevy_app::{CoreStage, Plugin};
use bevy_ecs::{
    schedule::{StageLabel, SystemStage},
    system::Resource,
    prelude::Entity,
};
use bevy_rapier3d::{
    prelude::{
        RapierConfiguration, RapierContext,
        RigidBody,
        Real,
    },
    rapier::prelude::{RigidBodyBuilder, RigidBodyHandle, Isometry},
};

use crate::systems;

#[derive(Resource, Default)]
struct ScratchRapier(RapierContext, RapierConfiguration);

pub struct CreatedBody {
    pub id: u64,
    pub body: RigidBody,
    pub transform: Isometry<Real>,
}

#[derive(Resource)]
pub struct LocalContext {
    pub physics_scale: Real,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
enum PhysicsStage {
    SyncBackend,
    Writeback,
}

pub enum Request {
    CreateBody(Vec<CreatedBody>),
}

pub enum Response {
    RigidBodyHandles(Vec<(u64, RigidBodyHandle)>),
    SimulationResult(()),
}

#[derive(Resource)]
pub struct RequestSender(pub Sender<Request>);
#[derive(Resource)]
pub struct ResponseReceiver(pub Receiver<Response>);

pub struct RapierPhysicsPlugin;

impl Plugin for RapierPhysicsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(RapierContext::default());
        app.insert_resource(LocalContext { physics_scale: 1.0 });

        app.add_stage_after(
            CoreStage::Update,
            PhysicsStage::SyncBackend,
            SystemStage::parallel()
                .with_system(systems::init_rigid_bodies)
        );

        app.add_stage_after(
            PhysicsStage::SyncBackend,
            PhysicsStage::Writeback,
            SystemStage::parallel()
                .with_system(systems::writeback),
        );

        let (req_tx, req_rx) = bounded(1);
        let (res_tx, res_rx) = bounded(1);

        app.insert_resource(ResponseReceiver(res_rx));
        app.insert_resource(RequestSender(req_tx));

        std::thread::spawn(move || {
            let mut context = RapierContext::default();

            while let Ok(req) = req_rx.recv() {
                match req {
                    Request::CreateBody(bodies) => {
                        log::info!("received bodies, {}", bodies.len());

                        let mut rbs = Vec::<(u64, RigidBodyHandle)>::new();

                        for body in bodies {
                            let mut builder = RigidBodyBuilder::new(body.body.into());

                            builder = builder.position(body.transform);

                            let rb = builder.build();

                            let handle = context.bodies.insert(rb);
                            rbs.push((body.id, handle));

                            context.entity2body.insert(Entity::from_bits(body.id), handle);
                        }

                        res_tx.send(Response::RigidBodyHandles(rbs)).unwrap();
                    }
                }
            }
        });
    }
}
