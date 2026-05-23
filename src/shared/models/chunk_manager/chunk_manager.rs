use bevy::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Resource)]
pub struct ChunkManager {
    pub loaded: HashMap<(i32, i32), Vec<Entity>>,
    pub last_cam_chunk: Option<(i32, i32)>,
    pub spawn_queue: VecDeque<(i32, i32)>,
    pub spawns_per_frame: usize,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded: HashMap::new(),
            last_cam_chunk: None,
            spawn_queue: VecDeque::new(),
            spawns_per_frame: 3,
        }
    }
}
