use std::collections::BTreeMap;

use bevy::{
    asset::{Assets, Handle},
    image::{Image, TextureAtlasBuilder},
    math::Rect,
};

use crate::Identifier;

#[derive(Default, Clone)]
pub struct AtlasManager {
    data: BTreeMap<Identifier, TextureData>, // Using BTreeMap instead of HashMap for garunteed ordering, potentially not needed
    error_texture: Option<Handle<Image>>,
    error_atlas_location: Option<Rect>,
    atlas_texture: Option<Handle<Image>>,
}

impl AtlasManager {
    pub fn set_error_texture(&mut self, texture: Handle<Image>) {
        self.error_texture = Some(texture);
    }

    pub fn add_data(&mut self, identifier: Identifier, texture: Handle<Image>) {
        self.data.insert(
            identifier,
            TextureData {
                texture,
                atlas_location: None,
            },
        );
    }

    pub fn remove_data(&mut self, identifier: &Identifier) {
        self.data.remove(identifier);
    }

    /// WARNING: This may invalidate existing chunks
    pub fn rebuild_atlas(&mut self, textures: &mut Assets<Image>) {
        let mut texture_atlas_builder = TextureAtlasBuilder::default();

        if let Some(error_texture) = &self.error_texture {
            let id = error_texture.id();
            texture_atlas_builder.add_texture(Some(id), textures.get(id).unwrap());
        }

        for texture_data in self.data.values() {
            let id = texture_data.texture.id();
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
            })
        }

        for (i, texture_data) in self.data.values_mut().enumerate() {
            let i = if self.error_texture.is_some() {
                i + 1
            } else {
                i
            };

            // Convert to 0.0 -> 1.0
            texture_data.atlas_location = Some(Rect {
                min: texture_atlas_layout.textures[i].as_rect().min
                    / texture_atlas_layout.size.as_vec2(),
                max: texture_atlas_layout.textures[i].as_rect().max
                    / texture_atlas_layout.size.as_vec2(),
            });
        }

        self.atlas_texture = Some(textures.add(texture));
    }

    /// Get UV location of texture in atlas
    pub fn atlas_location(&self, identifier: &Identifier) -> Option<Rect> {
        self.data.get(identifier)?.atlas_location
    }

    /// Get UV location of texture in atlas or error texture if not found
    pub fn atlas_location_or_error(&self, identifier: &Identifier) -> Rect {
        self.atlas_location(identifier).unwrap_or(
            self.error_atlas_location
                .expect("Error texture has not been definied"),
        )
    }

    pub fn atlas_texture(&self) -> Option<Handle<Image>> {
        self.atlas_texture.clone()
    }
}

#[derive(Clone)]
struct TextureData {
    texture: Handle<Image>,
    atlas_location: Option<Rect>,
}
