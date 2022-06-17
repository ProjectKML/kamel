use kamel_bevy::{
    app::{App, PluginGroup, PluginGroupBuilder},
    core::CorePlugin,
    input::InputPlugin,
    log::LogPlugin,
    window::{WindowDescriptor, WindowPlugin},
    winit::WinitPlugin
};

struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(LogPlugin::default());
        group.add(CorePlugin::default());
        group.add(InputPlugin::default());
        group.add(WindowPlugin::default());
        group.add(WinitPlugin::default());
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 1600.0,
            height: 900.0,
            title: "Kamel Game".to_string(),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
