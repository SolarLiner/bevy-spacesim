use bevy::prelude::*;

type Precision = i32;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.build().disable::<TransformPlugin>(),
            big_space::BigSpacePlugin::<Precision>::new(false),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
        ))
        .add_plugins((solar_system::SolarSystemPlugin::<Precision>::default(),))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server.load("scenes/solar.system.yaml"),
        ..default()
    });
}
