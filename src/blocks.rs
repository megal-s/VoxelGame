use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// TODO: Have a plugin handle this cleaner (including loading textures automatically...)

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Block(String);

impl Block {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn id(&self) -> &str {
        &self.0
    }
}

struct BlockData {
    texture: Handle<Image>,
    atlas_location: Option<Rect>,
}

#[derive(Resource, Default, Deref)]
pub struct BlockManagerResource(Arc<Mutex<BlockManager>>);

#[derive(Default)]
pub struct BlockManager {
    blocks: BTreeMap<Block, BlockData>, // Using BTreeMap instead of HashMap for garunteed ordering, potentially not needed

    error_texture: Option<Handle<Image>>,
    error_atlas_location: Option<Rect>,

    atlas_texture: Option<Handle<Image>>,
}

impl BlockManager {
    pub fn set_error_texture(&mut self, texture: Handle<Image>) {
        self.error_texture = Some(texture);
    }

    pub fn add_block(&mut self, block: Block, texture: Handle<Image>) {
        self.blocks.insert(
            block,
            BlockData {
                texture,
                atlas_location: None,
            },
        );
    }

    pub fn remove_block(&mut self, block: &Block) {
        self.blocks.remove(block);
    }

    /// WARNING: This may invalidate existing chunks
    pub fn rebuild_atlas(&mut self, textures: &mut Assets<Image>) {
        let mut texture_atlas_builder = TextureAtlasBuilder::default();

        if let Some(error_texture) = &self.error_texture {
            let id = error_texture.id();

            texture_atlas_builder.add_texture(Some(id), textures.get(id).unwrap());
        }

        for block_data in self.blocks.values() {
            let id = block_data.texture.id();

            texture_atlas_builder.add_texture(Some(id), textures.get(id).unwrap());
        }

        let (texture_atlas_layout, _texture_atlas_sources, texture) =
            texture_atlas_builder.build().unwrap();

        if self.error_texture.is_some() {
            self.error_atlas_location = Some(Rect {
                min: texture_atlas_layout.textures[0].as_rect().min
                    / texture_atlas_layout.size.as_vec2(),
                max: texture_atlas_layout.textures[0].as_rect().max
                    / texture_atlas_layout.size.as_vec2(),
            });
        }

        for (i, block_data) in self.blocks.values_mut().enumerate() {
            let i = if self.error_texture.is_some() {
                i + 1
            } else {
                i
            };

            // Convert to 0.0 -> 1.0
            block_data.atlas_location = Some(Rect {
                min: texture_atlas_layout.textures[i].as_rect().min
                    / texture_atlas_layout.size.as_vec2(),
                max: texture_atlas_layout.textures[i].as_rect().max
                    / texture_atlas_layout.size.as_vec2(),
            });
        }

        self.atlas_texture = Some(textures.add(texture));
    }

    /// Get location in atlas of the block texture (from 0.0 -> 1.0 for use in UVs)
    pub fn atlas_location(&self, block: &Block) -> Option<Rect> {
        self.blocks.get(block)?.atlas_location
    }

    pub fn atlas_location_or_error(&self, block: &Block) -> Rect {
        self.atlas_location(block).unwrap_or(
            self.error_atlas_location
                .expect("Error texture has not been defined"),
        )
    }

    pub fn atlas_texture(&self) -> Option<Handle<Image>> {
        self.atlas_texture.clone()
    }
}
