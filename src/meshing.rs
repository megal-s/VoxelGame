use bevy::{
    asset::RenderAssetUsages,
    math::IVec3,
    platform::collections::HashMap,
    render::mesh::{Indices, Mesh, PrimitiveTopology},
};

use crate::{
    blocks::BlockManager,
    chunk::{BlockGrid, CHUNK_BLOCK_COUNT, CHUNK_SIZE_I32, Chunk, ChunkGrid},
};

pub fn rebuild_chunk_meshes(
    chunk_grid: &ChunkGrid,
    block_manager: &BlockManager,
) -> HashMap<IVec3, Mesh> {
    let mut meshes = HashMap::new();
    for chunk in chunk_grid.chunks.values() {
        let Some(mesh) = rebuild_chunk_mesh(chunk_grid, block_manager, chunk) else {
            continue;
        };
        meshes.insert(chunk.position, mesh);
    }
    meshes
}

/// Will return [None] if there are no blocks to render
pub fn rebuild_chunk_mesh(
    chunk_grid: &ChunkGrid,
    block_manager: &BlockManager,
    chunk: &Chunk,
) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut uv_0 = Vec::new();
    let mut indices_offset = 0;

    for index in 0..CHUNK_BLOCK_COUNT {
        let Some(position) = BlockGrid::to_block_coordinates_from_index(index) else {
            continue;
        };
        let Some(block) = chunk.contents.get(position) else {
            continue;
        };

        let atlas_rect = block_manager.atlas_location_or_error(block);

        let block_position = IVec3::new(position.x as i32, position.y as i32, position.z as i32)
            + chunk.position * CHUNK_SIZE_I32;
        let (x, y, z) = (position.x as f32, position.y as f32, position.z as f32);

        // TOP FACE
        if chunk_grid.get_block(block_position + IVec3::Y).is_none() {
            positions.extend_from_slice(&[
                [x + -0.5, y + 0.5, z + -0.5],
                [x + 0.5, y + 0.5, z + -0.5],
                [x + 0.5, y + 0.5, z + 0.5],
                [x + -0.5, y + 0.5, z + 0.5],
            ]);
            normals.extend_from_slice(&[
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 2,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);

            indices_offset += 4;
        }
        // BOTTOM FACE
        if chunk_grid.get_block(block_position - IVec3::Y).is_none() {
            positions.extend_from_slice(&[
                [x + -0.5, y + -0.5, z + -0.5],
                [x + 0.5, y + -0.5, z + -0.5],
                [x + 0.5, y + -0.5, z + 0.5],
                [x + -0.5, y + -0.5, z + 0.5],
            ]);
            normals.extend_from_slice(&[
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, -1.0, 0.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 2,
                indices_offset + 3,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);
            indices_offset += 4;
        }
        // RIGHT FACE
        if chunk_grid.get_block(block_position + IVec3::X).is_none() {
            positions.extend_from_slice(&[
                [x + 0.5, y + -0.5, z + -0.5],
                [x + 0.5, y + -0.5, z + 0.5],
                [x + 0.5, y + 0.5, z + 0.5],
                [x + 0.5, y + 0.5, z + -0.5],
            ]);
            normals.extend_from_slice(&[
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 2,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);
            indices_offset += 4;
        }
        // LEFT FACE
        if chunk_grid.get_block(block_position - IVec3::X).is_none() {
            positions.extend_from_slice(&[
                [x + -0.5, y + -0.5, z + -0.5],
                [x + -0.5, y + -0.5, z + 0.5],
                [x + -0.5, y + 0.5, z + 0.5],
                [x + -0.5, y + 0.5, z + -0.5],
            ]);
            normals.extend_from_slice(&[
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
                [-1.0, 0.0, 0.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 2,
                indices_offset + 3,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);
            indices_offset += 4;
        }
        // BACK FACE
        if chunk_grid.get_block(block_position + IVec3::Z).is_none() {
            positions.extend_from_slice(&[
                [x + -0.5, y + -0.5, z + 0.5],
                [x + -0.5, y + 0.5, z + 0.5],
                [x + 0.5, y + 0.5, z + 0.5],
                [x + 0.5, y + -0.5, z + 0.5],
            ]);
            normals.extend_from_slice(&[
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 2,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);
            indices_offset += 4;
        }
        // FRONT FACE
        if chunk_grid.get_block(block_position - IVec3::Z).is_none() {
            positions.extend_from_slice(&[
                [x + -0.5, y + -0.5, z + -0.5],
                [x + -0.5, y + 0.5, z + -0.5],
                [x + 0.5, y + 0.5, z + -0.5],
                [x + 0.5, y + -0.5, z + -0.5],
            ]);
            normals.extend_from_slice(&[
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ]);
            indices.extend_from_slice(&[
                indices_offset,
                indices_offset + 1,
                indices_offset + 3,
                indices_offset + 1,
                indices_offset + 2,
                indices_offset + 3,
            ]);
            uv_0.extend_from_slice(&[
                [atlas_rect.min.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.min.y],
                [atlas_rect.max.x, atlas_rect.max.y],
                [atlas_rect.min.x, atlas_rect.max.y],
            ]);
            indices_offset += 4;
        }
    }

    if indices.is_empty() {
        return None;
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv_0)
        .with_inserted_indices(Indices::U32(indices)),
    )
}
