use std::sync::Arc;

use bevy::{asset::Handle, ecs::resource::Resource, image::Image};
use bevy_asset_loader::asset_collection::AssetCollection;
use serde::{Deserialize, Serialize};

use crate::{Identifier, atlas::AtlasManager};

#[derive(AssetCollection, Resource)]
pub struct BlockAssets {
    #[asset(path = "Error.png")]
    pub error: Handle<Image>,
    #[asset(path = "Stone.png")]
    pub stone: Handle<Image>,
    #[asset(path = "Dirt.png")]
    pub dirt: Handle<Image>,
}

#[derive(Default, Resource)]
pub struct BlockAtlasManager(pub Arc<AtlasManager>);

#[derive(Clone, Serialize, Deserialize)]
pub struct Block {
    pub identifier: Identifier,
}

impl Block {
    pub fn new(identifier: Identifier) -> Self {
        Self { identifier }
    }
}
