use crate::shared::enums::tile_type::tile_type::TileType;
use crate::shared::models::world_map::world_map::WorldMap;
use crate::shared::models::world_tile::world_tile::WorldTile;
use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMap>()
            .add_systems(Startup, (spawn_world, spawn_lighting));
    }
}

const WORLD_HALF: i32 = 32;
const TILE_SIZE: f32 = 1.0;
const HEIGHT_MIN: f32 = -15.0;
const HEIGHT_MAX: f32 = 15.0;
const SEA_LEVEL: f32 = -1.5;

// Perlin Noise

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn grad(hash: u8, x: f32, y: f32) -> f32 {
    match hash & 3 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

/// Classic 2D Perlin noise → [-1, 1]
fn perlin(x: f32, y: f32, perm: &[u8; 512]) -> f32 {
    let xi = x.floor() as i32 & 255;
    let yi = y.floor() as i32 & 255;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = fade(xf);
    let v = fade(yf);

    let aa = perm[(perm[xi as usize] as i32 + yi) as usize & 255];
    let ab = perm[(perm[xi as usize] as i32 + yi + 1) as usize & 255];
    let ba = perm[(perm[(xi + 1) as usize & 255] as i32 + yi) as usize & 255];
    let bb = perm[(perm[(xi + 1) as usize & 255] as i32 + yi + 1) as usize & 255];

    let x1 = lerp(grad(aa, xf, yf), grad(ba, xf - 1.0, yf), u);
    let x2 = lerp(grad(ab, xf, yf - 1.0), grad(bb, xf - 1.0, yf - 1.0), u);
    lerp(x1, x2, v)
}

/// Fractal Brownian Motion — stacks octaves for natural-looking terrain
fn fbm(x: f32, y: f32, perm: &[u8; 512]) -> f32 {
    let mut value = 0.0_f32;
    let mut amplitude = 0.5_f32;
    let mut frequency = 1.0_f32;

    for _ in 0..6 {
        value += perlin(x * frequency, y * frequency, perm) * amplitude;
        frequency *= 2.0; // lacunarity
        amplitude *= 0.5; // persistence / gain
    }
    value
}

/// Fisher-Yates shuffled permutation table from a u64 seed
fn build_perm(seed: u64) -> [u8; 512] {
    let mut p: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut rng = seed;
    for i in (1..256).rev() {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = ((rng >> 33) as usize) % (i + 1);
        p.swap(i, j);
    }
    let mut perm = [0u8; 512];
    for i in 0..512 {
        perm[i] = p[i & 255];
    }
    perm
}

// Height → biome mapping

fn height_to_tile(h: f32) -> TileType {
    if h < -8.0 {
        TileType::DeepWater
    } else if h < -2.0 {
        TileType::Water
    } else if h < 0.0 {
        TileType::Sand
    } else if h < 6.0 {
        TileType::Grass
    } else if h < 10.0 {
        TileType::Dirt
    } else if h < 13.0 {
        TileType::Stone
    } else {
        TileType::Snow
    }
}

// World spawn

fn spawn_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut world_map: ResMut<WorldMap>,
) {
    let perm = build_perm(42); // change seed for a different world layout

    // How zoomed-in the noise is — smaller = broader mountains
    let noise_scale = 0.045_f32;

    // Materials per biome
    let mat_deep_water = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.18, 0.48),
        perceptual_roughness: 0.05,
        reflectance: 0.9,
        ..default()
    });
    let mat_water = materials.add(StandardMaterial {
        base_color: Color::srgba(0.16, 0.42, 0.72, 0.82),
        perceptual_roughness: 0.05,
        reflectance: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let mat_sand = materials.add(StandardMaterial {
        base_color: Color::srgb(0.83, 0.76, 0.52),
        perceptual_roughness: 0.95,
        ..default()
    });
    let mat_grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.62, 0.22),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mat_dirt = materials.add(StandardMaterial {
        base_color: Color::srgb(0.46, 0.32, 0.18),
        perceptual_roughness: 1.0,
        ..default()
    });
    let mat_stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.50, 0.52),
        perceptual_roughness: 0.9,
        ..default()
    });
    let mat_snow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.95, 1.0),
        perceptual_roughness: 0.65,
        reflectance: 0.35,
        ..default()
    });

    let grass_scene: Handle<Scene> = asset_server.load("grass.glb#Scene0");

    for z in -WORLD_HALF..=WORLD_HALF {
        for x in -WORLD_HALF..=WORLD_HALF {
            // Sample fBm and remap to [HEIGHT_MIN, HEIGHT_MAX]
            let raw = fbm(x as f32 * noise_scale, z as f32 * noise_scale, &perm).clamp(-1.0, 1.0);

            let height = raw * ((HEIGHT_MAX - HEIGHT_MIN) / 2.0) + (HEIGHT_MAX + HEIGHT_MIN) / 2.0;

            let tile_type = height_to_tile(height);
            world_map.tiles.insert((x, z), (tile_type, height));

            // Submerged tiles rise to sea level so water surface is flat
            let render_y = height.max(SEA_LEVEL);

            // Taller slab at higher elevations for a stacked look
            let slab_h = (0.2 + height.abs() * 0.018).clamp(0.15, 0.8);

            let material = match tile_type {
                TileType::DeepWater => mat_deep_water.clone(),
                TileType::Water => mat_water.clone(),
                TileType::Sand => mat_sand.clone(),
                TileType::Grass => mat_grass.clone(),
                TileType::Dirt => mat_dirt.clone(),
                TileType::Stone => mat_stone.clone(),
                TileType::Snow => mat_snow.clone(),
            };

            let pos = Vec3::new(x as f32 * TILE_SIZE, render_y, z as f32 * TILE_SIZE);

            if tile_type == TileType::Grass {
                // GLB IS the grass block — no cuboid, just the model
                commands.spawn((
                    SceneRoot(grass_scene.clone()),
                    Transform::from_translation(pos).with_scale(Vec3::splat(1.0)), // tweak if your GLB is not 1x1 unit
                    WorldTile {
                        tile_type,
                        grid_pos: (x, z),
                        height,
                    },
                ));
            } else {
                let tile_mesh = meshes.add(Cuboid::new(TILE_SIZE, slab_h, TILE_SIZE));
                commands.spawn((
                    Mesh3d(tile_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(pos),
                    WorldTile {
                        tile_type,
                        grid_pos: (x, z),
                        height,
                    },
                ));
            }
        }
    }

    // Deep ocean floor so nothing is see-through from below
    let ocean_size = (WORLD_HALF * 2 + 4) as f32 * TILE_SIZE;
    let mat_ocean_floor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.10, 0.30),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(ocean_size, ocean_size))),
        MeshMaterial3d(mat_ocean_floor),
        Transform::from_translation(Vec3::new(0.0, HEIGHT_MIN - 0.2, 0.0)),
    ));
}

fn spawn_lighting(mut commands: Commands) {
    // Main sun — angled for dramatic isometric shadows across the terrain
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.96, 0.85),
            illuminance: 20_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            (-55.0_f32).to_radians(),
            (40.0_f32).to_radians(),
            0.0,
        )),
    ));

    // Sky bounce — soft blue fill from the opposite side
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.55, 0.65, 0.88),
        brightness: 450.0,
    });
}
