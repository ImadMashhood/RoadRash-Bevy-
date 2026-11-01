//! A surface keeps one image asset; changing dimensions replaces the handle as well as the texture.
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
pub fn upload_rgba(
    images: &mut Assets<Image>,
    handle: &mut Handle<Image>,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
) -> bool {
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    if images
        .get(&*handle)
        .is_none_or(|image| image.texture_descriptor.size != size)
    {
        let next = images.add(Image::new(
            size,
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ));
        images.remove(handle.id());
        *handle = next;
        true
    } else {
        if let Some(mut image) = images.get_mut(&*handle) {
            image.data = Some(pixels);
        }
        false
    }
}
