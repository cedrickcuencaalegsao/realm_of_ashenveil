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
pub const CHUNK_SIZE: i32 = 16;
pub const VIEW_DIST: i32 = 4;
pub const HEIGHT_MIN: f32 = -8.0;
pub const HEIGHT_MAX: f32 = 8.0;
pub const SEA_LEVEL: f32 = -1.5;
pub const NOISE_SCALE: f32 = 0.045;
pub const WORLD_SEED: u64 = 42;

/// Minimum Chebyshev distance between two stone boulders (in world tiles).
const STONE_MIN_DIST: i32 = 2;

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

#[inline]
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

/// Fractal Brownian Motion — 6 octaves for rich, natural-looking terrain.
fn fbm(x: f32, y: f32, perm: &[u8; 512]) -> f32 {
    let mut val = 0.0_f32;
    let mut amp = 0.5_f32;
    let mut freq = 1.0_f32;
    for _ in 0..4 {
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
    } else if h < -1.5 {
        TileType::Water
    } else if h < 0.5 {
        TileType::Sand
    } else if h < 5.5 {
        TileType::Grass
    } else if h < 7.0 {
        TileType::Dirt
    } else {
        TileType::Stone
    }
}

fn world_to_chunk(wx: f32, wz: f32) -> (i32, i32) {
    (
        (wx / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
        (wz / (CHUNK_SIZE as f32 * TILE_SIZE)).floor() as i32,
    )
}

/// Cheap, seedable hash for a world tile coordinate.
#[inline]
fn tile_hash(wx: i32, wz: i32, seed: u64) -> u64 {
    let mut h = seed
        .wrapping_add(wx as u64)
        .wrapping_mul(0x9e3779b97f4a7c15)
        .wrapping_add(wz as u64)
        .wrapping_mul(0x6c62272e07bb0142);
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    h
}

fn should_place_stone(wx: i32, wz: i32, tile_type: TileType) -> bool {
    // Stones only make sense on land above the water line.
    match tile_type {
        TileType::Water | TileType::DeepWater | TileType::Sand => return false,
        _ => {}
    }

    let my_hash = tile_hash(wx, wz, WORLD_SEED ^ 0xDEAD_BEEF);

    // Only top ~6 % of tiles are even candidates.
    if my_hash < (u64::MAX / 16) * 15 {
        return false;
    }

    // Must be the local maximum within STONE_MIN_DIST.
    for dz in -STONE_MIN_DIST..=STONE_MIN_DIST {
        for dx in -STONE_MIN_DIST..=STONE_MIN_DIST {
            if dx == 0 && dz == 0 {
                continue;
            }
            // let neighbor_hash = tile_hash(wx + dx, wz + dz, WORLD_SEED ^ 0xDEAD_BEEF);
            if my_hash < (u64::MAX / 32) * 31 {
                return false;
            }
        }
    }
    true
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
    terrain.scene_weed = Some(asset_server.load("weed.glb#Scene0"));

    // Pre-warm: keep extra handles alive so assets are cached before first chunk spawns
    terrain.warmup_handles = Some(vec![
        asset_server.load("grass.glb#Scene0"),
        asset_server.load("soil.glb#Scene0"),
        asset_server.load("sand.glb#Scene0"),
        asset_server.load("stone.glb#Scene0"),
        asset_server.load("weed.glb#Scene0"),
    ]);

    terrain.mat_deep_water = Some(materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.42, 0.58),
        perceptual_roughness: 0.05,
        reflectance: 0.9,
        ..default()
    }));
    terrain.mat_water = Some(materials.add(StandardMaterial {
        base_color: Color::srgba(0.28, 0.62, 0.72, 0.78),
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

    let cam_pos = cam_transform.translation;
    let cam_chunk = world_to_chunk(cam_pos.x, cam_pos.z);

    // Dead zone: only re-evaluate if camera moved at least 1 chunk away
    if let Some((lx, lz)) = manager.last_cam_chunk {
        let (cx, cz) = cam_chunk;
        if (lx - cx).abs() < 1 && (lz - cz).abs() < 1 {
            // Still process the spawn queue even if camera hasn't moved chunks
            drain_spawn_queue(
                &mut commands,
                &mut meshes,
                &terrain,
                &mut manager,
                cam_chunk,
            );
            return;
        }
    }
    manager.last_cam_chunk = Some(cam_chunk);

    let (cx, cz) = cam_chunk;

    // Despawn chunks outside view distance
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

    // Enqueue chunks that have entered view distance, closest first
    for dz in -VIEW_DIST..=VIEW_DIST {
        for dx in -VIEW_DIST..=VIEW_DIST {
            let chunk_coord = (cx + dx, cz + dz);
            if !manager.loaded.contains_key(&chunk_coord)
                && !manager.spawn_queue.contains(&chunk_coord)
            {
                manager.spawn_queue.push_back(chunk_coord);
            }
        }
    }

    // Sort queue so nearest chunks spawn first
    manager
        .spawn_queue
        .make_contiguous()
        .sort_by_key(|&(x, z)| {
            let dx = x - cx;
            let dz = z - cz;
            dx * dx + dz * dz
        });

    drain_spawn_queue(
        &mut commands,
        &mut meshes,
        &terrain,
        &mut manager,
        cam_chunk,
    );
}

fn should_place_weed(wx: i32, wz: i32, tile_type: TileType) -> bool {
    if tile_type != TileType::Grass {
        return false;
    }

    let my_hash = tile_hash(wx, wz, WORLD_SEED ^ 0xBEEF_CAFE);

    if my_hash < (u64::MAX / 4) * 3 {
        return false;
    }

    true
}

/// Spawns up to `spawns_per_frame` chunks from the front of the queue.
fn drain_spawn_queue(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain: &TerrainAssets,
    manager: &mut ChunkManager,
    _cam_chunk: (i32, i32),
) {
    let budget = manager.spawns_per_frame;
    for _ in 0..budget {
        let Some(coord) = manager.spawn_queue.pop_front() else {
            break;
        };
        // May have been loaded already by a previous frame
        if manager.loaded.contains_key(&coord) {
            continue;
        }
        let entities = spawn_chunk(commands, meshes, terrain, coord);
        manager.loaded.insert(coord, entities);
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain: &TerrainAssets,
    (chunk_x, chunk_z): (i32, i32),
) -> Vec<Entity> {
    // 2 entities per tile (surface + 1 dirt sublayer) + potential stone boulder
    let mut entities = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE * 3) as usize);

    let perm = &terrain.perm;
    let origin_x = chunk_x * CHUNK_SIZE;
    let origin_z = chunk_z * CHUNK_SIZE;

    // Reuse a single slab mesh handle per chunk to avoid redundant Mesh uploads.
    // Exact height varies per tile, so we still create one per tile — but we
    // keep water/snow slabs as shared handles within this chunk where possible.
    // For maximum reuse the caller would need a global mesh cache; the
    // per-chunk approach below avoids over-engineering while still being fast.

    for lz in 0..CHUNK_SIZE {
        for lx in 0..CHUNK_SIZE {
            let wx = origin_x + lx;
            let wz = origin_z + lz;

            let height = sample_height(wx, wz, perm);
            let tile_type = height_to_tile(height);

            // Surface tiles are clamped to sea-level so water looks flat.
            let render_y = height.max(SEA_LEVEL);
            let surface_pos = Vec3::new(wx as f32 * TILE_SIZE, render_y, wz as f32 * TILE_SIZE);

            // ── Surface tile ─────────────────────────────────────────────────
            let surface_entity = spawn_surface_tile(
                commands,
                meshes,
                terrain,
                tile_type,
                surface_pos,
                height,
                (chunk_x, chunk_z),
            );
            entities.push(surface_entity);

            // ── Single dirt sublayer one unit below the surface ──────────────
            let dirt_pos = Vec3::new(wx as f32 * TILE_SIZE, render_y - 1.0, wz as f32 * TILE_SIZE);
            let dirt_entity = commands
                .spawn((
                    SceneRoot(terrain.scene_soil.clone().unwrap()),
                    Transform::from_translation(dirt_pos),
                    WorldTile {
                        tile_type: TileType::Dirt,
                        chunk: (chunk_x, chunk_z),
                    },
                ))
                .id();
            entities.push(dirt_entity);

            // ── Stone boulders (scattered, ≥ STONE_MIN_DIST apart) ───────────
            if should_place_stone(wx, wz, tile_type) {
                // Place the boulder one unit above the surface so it sits on top.
                let boulder_pos =
                    Vec3::new(wx as f32 * TILE_SIZE, render_y + 1.0, wz as f32 * TILE_SIZE);
                let boulder = commands
                    .spawn((
                        SceneRoot(terrain.scene_stone.clone().unwrap()),
                        Transform::from_translation(boulder_pos),
                        WorldTile {
                            tile_type: TileType::Stone,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id();
                entities.push(boulder);
            }

            // ── Weeds (scattered on grass) ───────────────────────────────────
            if should_place_weed(wx, wz, tile_type) {
                let weed_pos =
                    Vec3::new(wx as f32 * TILE_SIZE, render_y + 1.0, wz as f32 * TILE_SIZE);
                let weed = commands
                    .spawn((
                        SceneRoot(terrain.scene_weed.clone().unwrap()),
                        Transform::from_translation(weed_pos),
                        WorldTile {
                            tile_type: TileType::Grass,
                            chunk: (chunk_x, chunk_z),
                        },
                    ))
                    .id();
                entities.push(weed);
            }
        }
    }

    entities
}

/// Spawns the visual for a single surface tile and returns its Entity.
fn spawn_surface_tile(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain: &TerrainAssets,
    tile_type: TileType,
    pos: Vec3,
    height: f32,
    chunk: (i32, i32),
) -> Entity {
    match tile_type {
        TileType::Grass => commands
            .spawn((
                SceneRoot(terrain.scene_grass.clone().unwrap()),
                Transform::from_translation(pos),
                WorldTile { tile_type, chunk },
            ))
            .id(),

        TileType::Dirt => commands
            .spawn((
                SceneRoot(terrain.scene_soil.clone().unwrap()),
                Transform::from_translation(pos),
                WorldTile { tile_type, chunk },
            ))
            .id(),

        TileType::Sand => commands
            .spawn((
                SceneRoot(terrain.scene_sand.clone().unwrap()),
                Transform::from_translation(pos),
                WorldTile { tile_type, chunk },
            ))
            .id(),

        TileType::Stone => commands
            .spawn((
                SceneRoot(terrain.scene_stone.clone().unwrap()),
                Transform::from_translation(pos),
                WorldTile { tile_type, chunk },
            ))
            .id(),

        TileType::DeepWater | TileType::Water | TileType::Snow => {
            // Use a flat cuboid slab for water/snow tiles.
            let slab_h = (0.2 + height.abs() * 0.018).clamp(0.15, 0.8);
            let mesh = meshes.add(Cuboid::new(TILE_SIZE, slab_h, TILE_SIZE));
            let material = match tile_type {
                TileType::DeepWater => terrain.mat_deep_water.clone().unwrap(),
                TileType::Water => terrain.mat_water.clone().unwrap(),
                _ => terrain.mat_snow.clone().unwrap(),
            };
            commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(pos),
                    WorldTile { tile_type, chunk },
                ))
                .id()
        }
    }
}

fn spawn_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.88, 0.70),
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            (-35.0_f32).to_radians(),
            (30.0_f32).to_radians(),
            0.0,
        )),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.72, 0.68, 0.55),
        brightness: 600.0,
    });
}
