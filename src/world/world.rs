use crate::shared::enums::tile_type::tile_type::TileType;
use crate::shared::models::chunk_manager::chunk_manager::ChunkManager;
use crate::shared::models::terrain_assets::terrain_assets::TerrainAssets;
use crate::shared::models::world_tile::world_tile::WorldTile;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkManager>()
            .init_resource::<TerrainAssets>()
            .add_systems(Startup, (setup_terrain_assets, spawn_lighting))
            .add_systems(Update, update_chunks);
    }
}

pub const TILE_SIZE: f32 = 1.0;
pub const CHUNK_SIZE: i32 = 16; // 16×16 tiles per chunk
pub const VIEW_DIST: i32 = 4; // chunks in each direction to keep loaded
pub const HEIGHT_MIN: f32 = -15.0;
pub const HEIGHT_MAX: f32 = 15.0;
pub const SEA_LEVEL: f32 = -1.5;
pub const NOISE_SCALE: f32 = 0.045;
pub const WORLD_SEED: u64 = 42;

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

fn fbm(x: f32, y: f32, perm: &[u8; 512]) -> f32 {
    let (mut val, mut amp, mut freq) = (0.0_f32, 0.5_f32, 1.0_f32);
    for _ in 0..6 {
        val += perlin(x * freq, y * freq, perm) * amp;
        freq *= 2.0;
        amp *= 0.5;
    }
    val
}

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

fn sample_height(wx: i32, wz: i32, perm: &[u8; 512]) -> f32 {
    let raw = fbm(wx as f32 * NOISE_SCALE, wz as f32 * NOISE_SCALE, perm).clamp(-1.0, 1.0);
    raw * ((HEIGHT_MAX - HEIGHT_MIN) / 2.0) + (HEIGHT_MAX + HEIGHT_MIN) / 2.0
}

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

fn world_to_chunk(wx: f32, wz: f32) -> (i32, i32) {
    (
        (wx / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
        (wz / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
    )
}

fn setup_terrain_assets(
    mut terrain: ResMut<TerrainAssets>,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    terrain.scene_grass = Some(asset_server.load("grass.glb#Scene0"));
    terrain.scene_soil = Some(asset_server.load("soil.glb#Scene0"));
    terrain.scene_sand = Some(asset_server.load("sand.glb#Scene0"));
    terrain.scene_stone = Some(asset_server.load("stone.glb#Scene0"));

    terrain.mat_deep_water = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.18, 0.48),
        perceptual_roughness: 0.05,
        reflectance: 0.9,
        ..default()
    }));
    terrain.mat_water = Some(materials.add(StandardMaterial {
        base_color: Color::srgba(0.16, 0.42, 0.72, 0.82),
        perceptual_roughness: 0.05,
        reflectance: 0.8,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }));
    terrain.mat_snow = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.95, 1.0),
        perceptual_roughness: 0.65,
        reflectance: 0.35,
        ..default()
    }));

    terrain.perm = build_perm(WORLD_SEED);
}

fn update_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain: Res<TerrainAssets>,
    mut manager: ResMut<ChunkManager>,
    cam_query: Query<&Transform, With<Camera3d>>,
) {
    let Ok(cam_transform) = cam_query.get_single() else {
        return;
    };

    // Work out which chunk the camera focus is over
    let cam_pos = cam_transform.translation;
    let cam_chunk = world_to_chunk(cam_pos.x, cam_pos.z);

    // Skip if the camera hasn't moved to a new chunk
    if manager.last_cam_chunk == Some(cam_chunk) {
        return;
    }
    manager.last_cam_chunk = Some(cam_chunk);

    let (cx, cz) = cam_chunk;

    let to_remove: Vec<(i32, i32)> = manager
        .loaded
        .keys()
        .filter(|&&(x, z)| (x - cx).abs() > VIEW_DIST || (z - cz).abs() > VIEW_DIST)
        .copied()
        .collect();

    for coord in to_remove {
        if let Some(entities) = manager.loaded.remove(&coord) {
            for entity in entities {
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    for dz in -VIEW_DIST..=VIEW_DIST {
        for dx in -VIEW_DIST..=VIEW_DIST {
            let chunk_coord = (cx + dx, cz + dz);
            if manager.loaded.contains_key(&chunk_coord) {
                continue;
            }

            let entities = spawn_chunk(&mut commands, &mut meshes, &terrain, chunk_coord);

            manager.loaded.insert(chunk_coord, entities);
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain: &TerrainAssets,
    (chunk_x, chunk_z): (i32, i32),
) -> Vec<Entity> {
    let mut entities = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE * 3) as usize);

    let perm = &terrain.perm;

    // Tile origin in world space
    let origin_x = chunk_x * CHUNK_SIZE;
    let origin_z = chunk_z * CHUNK_SIZE;

    for lz in 0..CHUNK_SIZE {
        for lx in 0..CHUNK_SIZE {
            let wx = origin_x + lx;
            let wz = origin_z + lz;

            let height = sample_height(wx, wz, perm);
            let tile_type = height_to_tile(height);
            let render_y = height.max(SEA_LEVEL);
            let pos = Vec3::new(wx as f32 * TILE_SIZE, render_y, wz as f32 * TILE_SIZE);
            let slab_h = (0.2 + height.abs() * 0.018).clamp(0.15, 0.8);

            let surface_entity = match tile_type {
                TileType::Grass => commands
                    .spawn((
                        SceneRoot(terrain.scene_grass.clone().unwrap()),
                        Transform::from_translation(pos).with_scale(Vec3::splat(1.0)),
                        WorldTile {
                            tile_type,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id(),

                TileType::Dirt => commands
                    .spawn((
                        SceneRoot(terrain.scene_soil.clone().unwrap()),
                        Transform::from_translation(pos).with_scale(Vec3::splat(1.0)),
                        WorldTile {
                            tile_type,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id(),

                TileType::Sand => commands
                    .spawn((
                        SceneRoot(terrain.scene_sand.clone().unwrap()),
                        Transform::from_translation(pos).with_scale(Vec3::splat(1.0)),
                        WorldTile {
                            tile_type,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id(),

                TileType::Stone => commands
                    .spawn((
                        SceneRoot(terrain.scene_stone.clone().unwrap()),
                        Transform::from_translation(pos).with_scale(Vec3::splat(1.0)),
                        WorldTile {
                            tile_type,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id(),

                TileType::DeepWater => {
                    let mesh = meshes.add(Cuboid::new(TILE_SIZE, slab_h, TILE_SIZE));
                    commands
                        .spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(terrain.mat_deep_water.clone().unwrap()),
                            Transform::from_translation(pos),
                            WorldTile {
                                tile_type,
                                chunk: (chunk_x, chunk_z),
                            },
                        ))
                        .id()
                }

                TileType::Water => {
                    let mesh = meshes.add(Cuboid::new(TILE_SIZE, slab_h, TILE_SIZE));
                    commands
                        .spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(terrain.mat_water.clone().unwrap()),
                            Transform::from_translation(pos),
                            WorldTile {
                                tile_type,
                                chunk: (chunk_x, chunk_z),
                            },
                        ))
                        .id()
                }

                TileType::Snow => {
                    let mesh = meshes.add(Cuboid::new(TILE_SIZE, slab_h, TILE_SIZE));
                    commands
                        .spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(terrain.mat_snow.clone().unwrap()),
                            Transform::from_translation(pos),
                            WorldTile {
                                tile_type,
                                chunk: (chunk_x, chunk_z),
                            },
                        ))
                        .id()
                }
            };
            entities.push(surface_entity);

            let sub_layers: &[f32] = match tile_type {
                TileType::Water | TileType::DeepWater => &[-1.0],
                _ => &[-1.0, -2.0],
            };

            for &offset in sub_layers {
                let sub_pos = Vec3::new(
                    wx as f32 * TILE_SIZE,
                    render_y + offset,
                    wz as f32 * TILE_SIZE,
                );
                let e = commands
                    .spawn((
                        SceneRoot(terrain.scene_soil.clone().unwrap()),
                        Transform::from_translation(sub_pos).with_scale(Vec3::splat(1.0)),
                        WorldTile {
                            tile_type: TileType::Dirt,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id();
                entities.push(e);
            }
        }
    }

    entities
}

fn spawn_lighting(mut commands: Commands) {
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

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.55, 0.65, 0.88),
        brightness: 450.0,
    });
}
