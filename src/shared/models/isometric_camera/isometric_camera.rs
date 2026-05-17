use bevy::prelude::*;

#[derive(Component)]
pub struct IsometricCamera {
    pub focus: Vec3,
    pub distance: f32,
}
