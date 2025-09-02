use bevy::{asset::Handle, ecs::resource::Resource, math::IVec3, platform::collections::HashMap, render::mesh::Mesh};

use crate::chunk::ChunkGrid;

#[derive(Default, Resource)]
pub struct Level {
    pub chunk_grid: ChunkGrid,
    pub meshes: HashMap<IVec3, Handle<Mesh>>,
}