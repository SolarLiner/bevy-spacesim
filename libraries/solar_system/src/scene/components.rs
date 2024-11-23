use bevy::prelude::*;

#[derive(Debug, Copy, Clone, Default, Reflect, Component)]
#[reflect(Component)]
pub struct SceneRoot;

#[derive(Debug, Copy, Clone, Default, Reflect, Component)]
#[reflect(Component)]
pub struct SceneCamera;