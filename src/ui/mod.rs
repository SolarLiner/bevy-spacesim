use crate::ui::planets::Planets;
use crate::Reparent;
use bevy::app::Plugins;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::{RunSystemOnce, SystemParam};
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::window::PrimaryWindow;
use bevy_blur_regions::{BlurRegionsCamera, BlurRegionsPlugin};
use bevy_egui::{EguiContext, EguiPlugin};
use egui::panel::TopBottomSide;
use egui::{containers, widgets, Align, TextBuffer, Ui};
use egui_plot::{PlotPoint, PlotPoints};
use solar_system::orbit::DrawOrbits;
use solar_system::scene::components::SceneCamera;

mod planets;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        if !app.is_plugin_added::<RenderDiagnosticsPlugin>() {
            app.add_plugins(RenderDiagnosticsPlugin);
        }

        app.add_plugins(BlurRegionsPlugin::default())
            .add_plugins(planets::PlanetsPlugin)
            .init_resource::<UiState>()
            .add_systems(Update, ui);
    }
}

#[derive(Default, Resource)]
struct UiState {
    topbar_expanded: bool,
}

#[derive(SystemParam)]
struct UiSystems<'w, 's> {
    state: ResMut<'w, UiState>,
    planets: Res<'w, Planets>,
    diagnostics: Res<'w, DiagnosticsStore>,
    draw_orbits: ResMut<'w, DrawOrbits>,
    time: ResMut<'w, Time<Virtual>>,
    q_camera_blur: Query<'w, 's, &'static mut BlurRegionsCamera<20>>,
    q_camera_entity: Query<'w, 's, Entity, With<SceneCamera>>,
    q_camera_parent: Query<'w, 's, &'static Parent, With<SceneCamera>>,
    commands: Commands<'w, 's>,
}

fn ui(mut this: UiSystems, mut q_egui: Query<&mut EguiContext, With<PrimaryWindow>>) {
    let Ok(mut egui) = q_egui.get_single_mut() else {
        return;
    };

    let ctx = egui.get_mut();
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    });
    let rect = egui::TopBottomPanel::new(TopBottomSide::Top, "toolbar")
        .frame(egui::Frame::none().fill(egui::Color32::from_black_alpha(64)))
        .show(ctx, |ui| {
            this.topbar_ui(ui);
        })
        .response
        .rect;
    if let Ok(mut blur_regions) = this.q_camera_blur.get_single_mut() {
        let scale_factor = ctx.options(|op| op.zoom_factor);
        let min = vec2(rect.min.x, rect.min.y) * scale_factor;
        let max = vec2(rect.max.x, rect.max.y) * scale_factor;
        blur_regions.blur(Rect::from_corners(min, max));
    }
}

impl<'w, 's> UiSystems<'w, 's> {
    fn topbar_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.parent_selector(ui);
            ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                ui.add_space(4.0);
                if self.state.topbar_expanded {
                    self.fps_display_history(ui);
                } else {
                    self.fps_display(ui);
                }
                ui.separator();
                self.draw_orbit_toggle(ui);
                self.speed_controls(ui);
            });
        });
    }

    fn parent_selector(&mut self, ui: &mut Ui) {
        let planet = self.get_planet();
        containers::ComboBox::new("planets", "Current planet")
            .selected_text(planet.as_deref().unwrap_or("unknown"))
            .show_ui(ui, |ui| {
                let planets = self
                    .get_planets()
                    .map(|(e, s)| (e, s.to_string()))
                    .collect::<Vec<_>>();
                for (e, s) in planets {
                    if ui.button(s).clicked() {
                        if let Ok(entity) = self.q_camera_entity.get_single() {
                            self.commands.entity(entity).add(Reparent::ToEntity(e));
                        }
                    }
                }
            });
    }

    fn speed_controls(&mut self, ui: &mut Ui) {
        let mut cur_speed = self.time.relative_speed();
        ui.with_layout(egui::Layout::left_to_right(Align::Min), |ui| {
            if ui.button("-").clicked() {
                self.time.set_relative_speed(cur_speed / 10.);
            }
            if ui
                .add(
                    widgets::DragValue::new(&mut cur_speed)
                        .suffix("x")
                        .range(0.1..=1000.0),
                )
                .changed()
            {
                self.time.set_relative_speed(cur_speed);
            };
            if ui.button("+").clicked() {
                self.time.set_relative_speed(cur_speed * 10.0);
            }
        });
    }

    fn draw_orbit_toggle(&mut self, ui: &mut Ui) {
        let mut draw = **self.draw_orbits;
        if ui.checkbox(&mut draw, "Draw orbits").changed() {
            **self.draw_orbits = draw;
        };
    }

    fn fps_display(&mut self, ui: &mut Ui) {
        let Some(fps) = self.diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) else {
            return;
        };
        if !fps.is_enabled {
            return;
        }
        if ui
            .label(
                fps.average()
                    .map(|v| format!("FPS: {v:2.1} Hz"))
                    .unwrap_or_else(|| String::from("N/A")),
            )
            .double_clicked()
        {
            self.state.topbar_expanded = true;
        }
    }

    fn fps_display_history(&mut self, ui: &mut Ui) {
        let Some(fps) = self.diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) else {
            return;
        };
        if !fps.is_enabled {
            return;
        }
        let response = egui_plot::Plot::new("fps_history")
            .width(fps.history_len() as _)
            .allow_boxed_zoom(false)
            .allow_double_click_reset(false)
            .allow_scroll(false)
            .allow_drag(false)
            .allow_zoom(false)
            .show_x(false)
            .show_grid(false)
            .include_y(0.0)
            .show(ui, |ui| {
                ui.line(egui_plot::Line::new(PlotPoints::Owned(
                    fps.values()
                        .enumerate()
                        .map(|(i, &v)| PlotPoint::new(i as f64, v))
                        .collect::<Vec<_>>(),
                )));
            })
            .response;
        if response.double_clicked() {
            self.state.topbar_expanded = false;
        }
    }

    fn get_planets(&self) -> impl Iterator<Item = (Entity, &str)> {
        self.planets.iter()
    }

    fn get_planet(&self) -> Option<String> {
        for parent in &self.q_camera_parent {
            if let Some(name) = self.planets.get_name(**parent) {
                return Some(name.to_string());
            }
        }
        None
    }
}
