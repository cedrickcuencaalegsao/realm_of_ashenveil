use bevy::prelude::*;

#[derive(Resource)]
pub struct TerrainAssets {
    pub scene_grass: Option<Handle<Scene>>,
    pub scene_soil: Option<Handle<Scene>>,
    pub scene_sand: Option<Handle<Scene>>,
    pub scene_stone: Option<Handle<Scene>>,
    pub mat_deep_water: Option<Handle<StandardMaterial>>,
    pub mat_water: Option<Handle<StandardMaterial>>,
    pub mat_snow: Option<Handle<StandardMaterial>>,
    pub warmup_handles: Option<Vec<Handle<Scene>>>,
    pub scene_weed: Option<Handle<Scene>>,
    pub perm: [u8; 512],
}

impl Default for TerrainAssets {
    fn default() -> Self {
        Self {
            scene_grass: None,
            scene_soil: None,
            scene_sand: None,
            scene_stone: None,
            mat_deep_water: None,
            mat_water: None,
            mat_snow: None,
            warmup_handles: None,
            scene_weed: None,
            perm: [0u8; 512],
        }
    }
}
