use bevy_app::Plugin;
use bevy_asset::AddAsset;
use bevy_render::{
    camera::CameraPlugin, mesh::MeshPlugin, render_resource::Shader, texture::ImagePlugin,
    view::ViewPlugin,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
//        app.add_plugin(bevy_render::RenderPlugin::default())
//            .add_plugin(bevy_render::texture::ImagePlugin::default())
//            .add_plugin(bevy_render::pipelined_rendering::PipelinedRenderingPlugin::default());
        app.add_asset::<Shader>()
            .add_plugin(CameraPlugin)
            .add_plugin(ViewPlugin)
            .add_plugin(MeshPlugin)
            .add_plugin(ImagePlugin::default());
    }
}
