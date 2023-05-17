use bevy_app::Plugin;
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_time::Time;
use bevy_window::Windows;
use shared::settings::Settings;

#[derive(Default)]
pub struct NetworkLog {
    pub raw: u64,
    pub compressed: u64,
}

#[derive(Default)]
pub struct TimeLog {
    pub compress: u32,
    pub decompress: u32,
}

#[derive(Default, Resource)]
pub struct PluginLog {
    pub physics_time: u32,
    pub network_time: u32,
    pub uplink: NetworkLog,
    pub downlink: NetworkLog,
    pub client: TimeLog,
    pub server: TimeLog,
}

#[derive(Resource)]
struct InternalLog {
    frame_count: u64,
    start: std::time::Instant,
}

#[derive(Default)]
pub struct BenchPlugin;

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(InternalLog {
            frame_count: 0,
            start: std::time::Instant::now(),
        })
        .insert_resource(PluginLog::default());

        app.add_system_set_to_stage(
            bevy_app::CoreStage::Last,
            bevy_ecs::schedule::SystemSet::new()
                .with_system(log)
                .with_system(close_if_bench_finished),
        );

        println!("timestamp,fps,network_time,physics_time,uplink_raw,uplink_compressed,downlink_raw,downlink_compressed");
    }
}

fn close_if_bench_finished(time: Res<Time>, mut windows: ResMut<Windows>, settings: Res<Settings>) {
    if time.elapsed_seconds_wrapped() > settings.bench_length {
        windows.iter_mut().for_each(|window| window.close());
    }
}

fn log(time: Res<Time>, mut internal_log: ResMut<InternalLog>, mut log: ResMut<PluginLog>) {
    let log = std::mem::replace(&mut *log, PluginLog::default());

    let fps = if time.delta_seconds() == 0.0 { 0.0 } else { 1.0 / time.delta_seconds() };

    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        internal_log.start.elapsed().as_millis(),
        internal_log.frame_count,
        fps,
        log.physics_time,
        log.network_time,
        log.uplink.raw,
        log.uplink.compressed,
        log.downlink.raw,
        log.downlink.compressed,
        log.client.compress,
        log.client.decompress,
        log.server.compress,
        log.server.decompress,
    );

    internal_log.frame_count += 1;
}
