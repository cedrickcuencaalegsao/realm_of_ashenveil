use crate::shared::models::isometric_camera::isometric_camera::IsometricCamera;
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_isometric_camera)
            .add_systems(Update, camera_pan);
    }
}

fn spawn_isometric_camera(mut commands: Commands) {
    // Isometric: 45° yaw, 35.264° pitch (true isometric is ~35.26°, we use 35°)
    let focus = Vec3::ZERO;
    let distance = 30.0; // camera distance from focus

    // Yaw 45 degrees, pitch 35 degrees
    let yaw = 45.0_f32.to_radians();
    let pitch = 35.0_f32.to_radians();

    let offset = Vec3::new(
        distance * pitch.cos() * yaw.sin(),
        distance * pitch.sin(),
        distance * pitch.cos() * yaw.cos(),
    );

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(focus + offset).looking_at(focus, Vec3::Y),
        IsometricCamera { focus, distance },
    ));

    // Ambient light for the world
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 800.0,
    });
}

fn camera_pan(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut IsometricCamera)>,
) {
    let speed = 8.0;
    let dt = time.delta_secs();

    for (mut transform, mut iso_cam) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        // WASD / Arrow keys — move along isometric axes
        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            direction += Vec3::new(-1.0, 0.0, -1.0).normalize();
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            direction += Vec3::new(1.0, 0.0, 1.0).normalize();
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            direction += Vec3::new(-1.0, 0.0, 1.0).normalize();
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            direction += Vec3::new(1.0, 0.0, -1.0).normalize();
        }

        // Zoom with Q/E
        if keyboard.pressed(KeyCode::KeyQ) {
            iso_cam.distance = (iso_cam.distance - speed * dt).max(5.0);
        }
        if keyboard.pressed(KeyCode::KeyE) {
            iso_cam.distance = (iso_cam.distance + speed * dt).min(50.0);
        }

        if direction != Vec3::ZERO {
            iso_cam.focus += direction * speed * dt;
        }

        // Recompute camera position from focus + fixed isometric angles
        let yaw = 45.0_f32.to_radians();
        let pitch = 35.0_f32.to_radians();
        let d = iso_cam.distance;

        let offset = Vec3::new(
            d * pitch.cos() * yaw.sin(),
            d * pitch.sin(),
            d * pitch.cos() * yaw.cos(),
        );

        transform.translation = iso_cam.focus + offset;
        *transform = transform.looking_at(iso_cam.focus, Vec3::Y);
    }
}
