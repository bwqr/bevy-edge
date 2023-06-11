use std::{net::SocketAddrV4, io::Write};

use bevy_app::{Plugin, CoreStage};
use bevy_ecs::{system::{Res, Resource}, prelude::EventWriter};
use bevy_input::{prelude::KeyCode, ButtonState, keyboard::KeyboardInput};
use bevy_log::prelude::*;
use crossbeam::channel::{Receiver, bounded, Sender};

#[derive(Resource)]
struct InputChannel(Receiver<Vec<KeyboardInput>>);

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let (tx, rx) = bounded(1);

        std::thread::spawn(move || read_remote(tx));

        app
            .insert_resource(InputChannel(rx))
            .add_plugin(bevy_input::InputPlugin::default())
            .add_system_to_stage(
                CoreStage::PreUpdate,
                poll_input_channel,
            );
    }
}

fn poll_input_channel(rx: Res<InputChannel>, mut keyboard_input: EventWriter<KeyboardInput>) {
    if let Ok(events) = rx.0.try_recv() {
        for event in events {
            keyboard_input.send(event);
        }
    }
}

fn read_remote(tx: Sender<Vec<KeyboardInput>>) {
    let srv = std::net::TcpListener::bind("0.0.0.0:4001".parse::<SocketAddrV4>().unwrap()).unwrap();

    while let Err(e) = run_input_server(&tx, &srv) {
        error!(e);
    }
}

fn run_input_server(tx: &Sender<Vec<KeyboardInput>>, srv: &std::net::TcpListener) -> Result<(), String> {
    let (mut stream, _) = srv.accept()
        .map_err(|e| format!("could not accept incoming request, {e:?}"))?;

    debug!("a client is connected to input server");

    loop {
        let span = info_span!("read_remote", name = "read_remote").entered();

        stream.write(&[0])
            .map_err(|e| format!("could not send sync message, {e:?}"))?;

        let events: Vec<(KeyCode, ButtonState)> = bincode::deserialize_from(&stream)
            .map_err(|e| format!("could not deserialize keycodes, {e:?}"))?;

        drop(span);

        tx.send(events.iter().map(|event| KeyboardInput { scan_code: 0, key_code: Some(event.0), state: event.1 }).collect())
            .map_err(|e| format!("could not send keyboard events to receiver, {e:?}"))?;
    }
}
