use std::sync::{RwLock, Weak};

use bevy::{
    asset::RenderAssetUsages,
    render::mesh::{Indices, Mesh, PrimitiveTopology},
};

use crate::{
    atlas::AtlasManager,
    chunk::{self, Chunk, SIZE_USIZE, Z_INDEX_USIZE},
};

/// Will return `None` if either [`Weak`] was invalidated while generating and `Some(None)` if the mesh would have been empty
pub fn build_mesh(
    chunk: Weak<RwLock<Chunk>>,
    atlas_manager: Weak<AtlasManager>,
) -> Option<Option<Mesh>> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut uv_0 = Vec::new();
    let mut indices_offset = 0;

    for index in 0..chunk::CONTENTS_SIZE {
        let atlas_rect = {
            let rw_lock = chunk.upgrade()?;
            let Some(ref block) = rw_lock.read().expect("Chunk rw poisoned").contents[index] else {
                continue;
            };
            atlas_manager
                .upgrade()?
                .atlas_location_or_error(&block.identifier)
        };

        let (x, y, z) = {
            let block_position = Chunk::to_block_coordinates_from_index(index).unwrap();
            (
                block_position.x as f32,
                block_position.y as f32,
                block_position.z as f32,
            )
        };

        // May be worth storing chunk.upgrade() as a local variable instead of calling Weak::upgrade for each face
        // SIZE_USIZE moves the index by 1 on the y axis
        // Z_INDEX_USIZE moves the index by 1 on the z axis
        // TOP FACE
        if index / SIZE_USIZE % SIZE_USIZE != SIZE_USIZE - 1
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index + SIZE_USIZE)
                .is_none_or(|block| block.is_none())
        {
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
        if index / SIZE_USIZE % SIZE_USIZE != 0
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index - SIZE_USIZE)
                .is_none_or(|block| block.is_none())
        {
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
        if index % SIZE_USIZE != SIZE_USIZE - 1
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index + 1)
                .is_none_or(|block| block.is_none())
        {
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
        if index % SIZE_USIZE != 0
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index - 1)
                .is_none_or(|block| block.is_none())
        {
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
        if index / Z_INDEX_USIZE != SIZE_USIZE - 1
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index + Z_INDEX_USIZE)
                .is_none_or(|block| block.is_none())
        {
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
        if index / Z_INDEX_USIZE != 0
            && chunk
                .upgrade()?
                .read()
                .expect("Chunk rw poisoned")
                .contents
                .get(index - Z_INDEX_USIZE)
                .is_none_or(|block| block.is_none())
        {
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
        return Some(None);
    }

    Some(Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv_0)
        .with_inserted_indices(Indices::U32(indices)),
    ))
}
