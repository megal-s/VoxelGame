use bevy::prelude::*;

#[derive(Resource)]
pub struct BlockTextureAtlas {
    pub layout: TextureAtlasLayout,
    pub texture: Handle<Image>,
}

pub fn create_block_atlas(
    mut commands: Commands,
    block_assets: Res<crate::BlockAssets>,
    textures: ResMut<Assets<Image>>,
) {
    let mut texture_atlas_builder = TextureAtlasBuilder::default();

    for block in [&block_assets.error, &block_assets.stone, &block_assets.dirt] {
        let id = block.id();

        texture_atlas_builder.add_texture(Some(id), textures.get(id).unwrap());
    }

    let (texture_atlas_layout, _texture_atlas_sources, texture) =
        texture_atlas_builder.build().unwrap();

    let texture_handle = textures.into_inner().add(texture);

    commands.insert_resource(BlockTextureAtlas {
        layout: texture_atlas_layout,
        texture: texture_handle,
    });

    commands.set_state(crate::GameState::InGame);
}
