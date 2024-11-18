use bevy::prelude::*;

mod manifest;
mod processor;

pub use manifest::MaterialSource;

pub struct PlanetScenePlugin;

impl Plugin for PlanetScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaticAssets>()
            .add_plugins(processor::PlanetSceneProcessorPlugin)
            .register_type::<MaterialSource>()
            .observe(replace_material);
    }
}

#[derive(Resource)]
struct StaticAssets {
    sphere: Handle<Mesh>,
}

impl FromWorld for StaticAssets {
    fn from_world(world: &mut World) -> Self {
        let sphere = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(1.0).mesh().ico(16).unwrap());
        Self { sphere }
    }
}

fn replace_material(
    trigger: Trigger<OnAdd, MaterialSource>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    static_assets: Res<StaticAssets>,
    query: Query<&MaterialSource>,
) {
    let entity = trigger.entity();
    let source = query.get(entity).unwrap();
    info!("Replacing material for {entity}");
    let material = match source {
        MaterialSource::Inline(material) => {
            materials.add(processor::create_planet_material(material))
        }
    };
    commands
        .entity(entity)
        .insert((static_assets.sphere.clone(), material))
        .remove::<MaterialSource>();
}
