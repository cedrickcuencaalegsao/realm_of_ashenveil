use crate::shared::enums::tile_type::tile_type::TileType;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct WorldMap {
    pub tiles: HashMap<(i32, i32), (TileType, f32)>,
}
