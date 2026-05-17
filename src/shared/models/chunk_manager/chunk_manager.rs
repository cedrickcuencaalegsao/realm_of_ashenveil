use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ChunkManager {
    /// chunk coord → list of spawned entity ids
    pub loaded: HashMap<(i32, i32), Vec<Entity>>,
    /// last camera chunk position — only re-evaluate when it changes
    pub last_cam_chunk: Option<(i32, i32)>,
}
