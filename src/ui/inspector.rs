use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_blur_regions::EguiWindowBlurExt;
use bevy_egui::EguiContext;
use bevy_inspector_egui::bevy_inspector;
use egui::containers;

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

    containers::Window::new("Inspector")
        .frame(super::default_blurry_frame())
        .show_with_blur(ctx.get_mut(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                bevy_inspector::ui_for_world_entities(world, ui);
            });
        });
}
