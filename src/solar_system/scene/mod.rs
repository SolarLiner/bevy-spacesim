use bevy::asset::saver::AssetSaver;
use bevy::asset::transformer::AssetTransformer;
use bevy::asset::{AssetLoader, AsyncWriteExt};
use bevy::prelude::*;
use bevy::utils::ConditionalSendFuture;
use big_space::BigSpaceCommands;
use serde::{Deserialize, Serialize};

mod processor;
mod manifest;

pub struct PlanetScenePlugin;

impl Plugin for PlanetScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(processor::PlanetSceneProcessorPlugin);
    }
}
