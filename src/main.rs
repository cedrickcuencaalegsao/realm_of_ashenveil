mod camera;
mod shared;
mod world;

use bevy::prelude::*;
use camera::CameraPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Realm of Ashenveil".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((WorldPlugin, CameraPlugin))
        .run();
}
