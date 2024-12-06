use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_blur_regions::EguiWindowBlurExt;
use bevy_egui::EguiContext;
use bevy_inspector_egui::{bevy_inspector, reflect_inspector};
use egui::containers;
use postprocessing::lens_flares::LensFlare;

pub struct Plugin;

impl bevy::prelude::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, inspector_ui);
    }
}

fn inspector_ui(world: &mut World) {
    let Ok(mut ctx) = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .get_single_mut(world)
        .map(|ctx| ctx.clone())
    else {
        return;
    };

    containers::Window::new("Lens Flare")
        .frame(super::default_blurry_frame())
        .show_with_blur(ctx.get_mut(), |ui| {
            let Some((entity, mut lens_flare)) = world
                .query::<(Entity, &mut LensFlare)>()
                .iter_mut(world)
                .map(|(e, c)| (e, c.clone()))
                .next()
            else {
                ui.disable();
                ui.label("No Lens Flare");
                return;
            };
            let changed = {
                let type_registry = world.resource::<AppTypeRegistry>();
                let type_registry = type_registry.read();
                reflect_inspector::ui_for_value(&mut lens_flare, ui, &type_registry)
            };

            if changed {
                world.entity_mut(entity).insert(lens_flare);
            }
        });

    containers::Window::new("Inspector")
        .frame(super::default_blurry_frame())
        .show_with_blur(ctx.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                bevy_inspector::ui_for_world_entities(world, ui);
            });
        });
}
