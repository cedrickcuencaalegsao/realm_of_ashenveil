use crate::shared::enums::tile_type::tile_type::TileType;
use bevy::prelude::*;

#[derive(Component)]
pub struct WorldTile {
    #[allow(dead_code)]
    pub tile_type: TileType,
    #[allow(dead_code)]
    pub grid_pos: (i32, i32),
    #[allow(dead_code)]
    pub height: f32,
}
