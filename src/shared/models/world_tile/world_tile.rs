use crate::shared::enums::tile_type::tile_type::TileType;
use bevy::prelude::*;

#[derive(Component)]
#[allow(dead_code)]
pub struct WorldTile {
    pub tile_type: TileType,
    pub chunk: (i32, i32),
}
