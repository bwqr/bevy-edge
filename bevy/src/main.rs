use bevy_app::App;
use bevy_ecs::system::Res;
use bevy_input::{prelude::KeyCode, Input};
use bevy_log::{Level, LogSettings};
use console::Term;

mod input;

fn move_paddle(keyboard_input: Res<Input<KeyCode>>, term: Res<Term>) {
    if keyboard_input.pressed(KeyCode::W) {
       term.write_str("going up").unwrap();
    } else if keyboard_input.pressed(KeyCode::D) {
        term.write_str("going right").unwrap();
    } else if keyboard_input.pressed(KeyCode::S) {
        term.write_str("going down").unwrap();
    } else if keyboard_input.pressed(KeyCode::A) {
        term.write_str("going left").unwrap();
    }
}

fn main() {
    let term = Term::stdout();

    App::new()
         .insert_resource(LogSettings {
             level: Level::TRACE,
             ..Default::default()
         })
        .insert_resource(bevy_app::ScheduleRunnerSettings {
            run_mode: bevy_app::RunMode::Loop {
                wait: Some(std::time::Duration::from_millis(100)),
            },
        })
        .insert_resource(term)
        .add_plugin(bevy_log::LogPlugin::default())
        .add_plugin(bevy_core::CorePlugin::default())
        .add_plugin(bevy_time::TimePlugin::default())
        .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
        .add_plugin(input::InputPlugin)
        .add_system(move_paddle)
        .run();
}
