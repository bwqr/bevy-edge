use bevy_app::Plugin;
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_time::Time;
use bevy_window::Windows;
use shared::settings::Settings;

#[derive(Default)]
pub struct NetworkLog {
    pub compressed: u64,
    pub raw: u64,
}

#[derive(Resource)]
pub struct Log {
    start: std::time::Instant,
    physics_time: Option<u128>,
    uplink: Option<NetworkLog>,
    downlink: Option<NetworkLog>,
}

impl Log {
    pub fn update_uplink(&mut self, network_log: NetworkLog) {
        self.uplink = Some(network_log);
    }

    pub fn update_downlink(&mut self, network_log: NetworkLog) {
        self.downlink = Some(network_log);
    }

    pub fn update_physics_time(&mut self, time: u128) {
        self.physics_time = Some(time);
    }
}

#[derive(Default)]
pub struct BenchPlugin;

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(Log {
            start: std::time::Instant::now(),
            physics_time: None,
            uplink: None,
            downlink: None,
        });

        app.add_system_set_to_stage(
            bevy_app::CoreStage::Last,
            bevy_ecs::schedule::SystemSet::new()
                .with_system(log)
                .with_system(close_if_bench_finished),
        );

        println!("timestamp,fps,physics_time,uplink_raw,uplink_compressed,downlink_raw,downlink_compressed");
    }
}

fn close_if_bench_finished(time: Res<Time>, mut windows: ResMut<Windows>, settings: Res<Settings>) {
    if time.elapsed_seconds_wrapped() > settings.bench_length {
        windows.iter_mut().for_each(|window| window.close());
    }
}

fn log(time: Res<Time>, mut log: ResMut<Log>) {
    let timestamp = std::time::Instant::now()
        .duration_since(log.start)
        .as_millis();

    let uplink = log.uplink.take().unwrap_or_default();
    let downlink = log.downlink.take().unwrap_or_default();
    let fps = if time.delta_seconds() == 0.0 { 0.0 } else { 1.0 / time.delta_seconds() };

    println!(
        "{},{},{},{},{},{},{}",
        timestamp,
        fps,
        log.physics_time.take().unwrap_or_default(),
        uplink.raw,
        uplink.compressed,
        downlink.raw,
        downlink.compressed
    );
}
