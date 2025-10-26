use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../frontend/dist"]
pub struct StaticFiles;

