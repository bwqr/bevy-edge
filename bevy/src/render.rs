use bevy_app::Plugin;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app
            .add_plugin(bevy_render::RenderPlugin::default());
    }
}
