use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use solar_system::body::PlanetaryBody;
use std::collections::HashMap;

pub(super) struct PlanetsPlugin;

impl Plugin for PlanetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Planets>()
            .add_observer(on_added_planet)
            .add_observer(on_removed_planet);
    }
}

fn on_added_planet(
    trigger: Trigger<OnAdd, PlanetaryBody>,
    mut planets: ResMut<Planets>,
    q_name: Query<&Name>,
) {
    let name = q_name
        .get(trigger.entity())
        .map(|name| name.as_str().to_string())
        .unwrap_or_else(|_| format!("Planet {}", trigger.entity()));
    planets.add_planet(trigger.entity(), name);
}

fn on_removed_planet(trigger: Trigger<OnRemove, PlanetaryBody>, mut planets: ResMut<Planets>) {
    planets.remove_planet(trigger.entity());
}

#[derive(Debug, Default, Resource, Reflect)]
#[reflect(Resource)]
pub(super) struct Planets {
    index: EntityHashMap<String>,
    rev_index: HashMap<String, Entity>,
    order: Vec<Entity>,
}

impl Planets {
    pub(super) fn get_name(&self, entity: Entity) -> Option<&str> {
        self.index.get(&entity).map(|s| s.as_str())
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (Entity, &str)> {
        self.order
            .iter()
            .map(|e| (*e, self.index.get(e).unwrap().as_str()))
    }

    fn add_planet(&mut self, entity: Entity, name: String) {
        self.index.insert(entity, name.clone());
        self.rev_index.insert(name, entity);
        self.order.push(entity);
    }

    fn remove_planet(&mut self, entity: Entity) {
        if let Some(name) = self.index.remove(&entity) {
            self.rev_index.remove(&name);
        }
        if let Some(pos) = self.order.iter().position(|e| *e == entity) {
            self.order.remove(pos);
        }
    }
}
