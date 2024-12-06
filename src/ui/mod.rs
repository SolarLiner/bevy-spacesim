use crate::app::Reparent;
use crate::ui::planets::Planets;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
use bevy::math::vec2;
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::window::PrimaryWindow;
use bevy_blur_regions::{BlurRegionsCamera, BlurRegionsPlugin, EguiWindowBlurExt};
use bevy_egui::{EguiContext, EguiPlugin};
use chrono::{DateTime, Datelike, Days, Months, TimeDelta, Timelike, Utc};
use egui::panel::TopBottomSide;
use egui::{containers, emath, widgets, Align, FontId, Ui};
use egui_plot::{PlotPoint, PlotPoints};
use solar_system::body::PlanetaryBody;
use solar_system::mjd::Mjd;
use solar_system::orbit::DrawOrbits;
use solar_system::scene::components::SceneCamera;
use solar_system::scene::distance::{Distance, DistanceUnit};
use starrynight::Star;
use std::ops;

mod inspector;
mod planets;

#[derive(Default)]
pub struct UiPlugin {
    pub with_inspector: bool,
}

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

        app.add_plugins(BlurRegionsPlugin::<20>)
            .add_plugins(planets::PlanetsPlugin)
            .init_resource::<UiState>()
            .add_systems(Update, ui);

        if self.with_inspector {
            app.add_plugins(inspector::Plugin);
        }
    }
}

#[derive(Resource)]
struct UiState {
    topbar_expanded: bool,
    date_window_opened: bool,
    show_labels: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            topbar_expanded: false,
            date_window_opened: false,
            show_labels: true,
        }
    }
}

#[derive(SystemParam)]
struct UiSystems<'w, 's> {
    state: ResMut<'w, UiState>,
    planets: Res<'w, Planets>,
    diagnostics: Res<'w, DiagnosticsStore>,
    draw_orbits: ResMut<'w, DrawOrbits>,
    mjd: ResMut<'w, Time<Mjd>>,
    time: ResMut<'w, Time<Virtual>>,
    q_camera_blur: Query<'w, 's, &'static mut BlurRegionsCamera<20>>,
    q_camera_entity: Query<'w, 's, Entity, With<SceneCamera>>,
    q_camera_parent: Query<'w, 's, &'static Parent, With<SceneCamera>>,
    q_camera_transform:
        Query<'w, 's, (&'static GlobalTransform, &'static Camera), With<SceneCamera>>,
    q_planetary_bodies:
        Query<'w, 's, (&'static GlobalTransform, Option<&'static Name>), With<PlanetaryBody>>,
    q_stars: Query<'w, 's, (&'static GlobalTransform, &'static Star, &'static Name)>,
    commands: Commands<'w, 's>,
}

fn ui(mut this: UiSystems, mut q_egui: Query<&mut EguiContext, With<PrimaryWindow>>) {
    let Ok(mut egui) = q_egui.get_single_mut() else {
        return;
    };

    let ctx = egui.get_mut();
    this.toplevel(ctx);
}

impl UiSystems<'_, '_> {
    fn toplevel(&mut self, ctx: &egui::Context) {
        self.topbar(ctx);
        self.date_time_window(ctx);
        self.bodies_on_screen(ctx);
    }

    fn topbar(&mut self, ctx: &egui::Context) {
        let rect = egui::TopBottomPanel::new(TopBottomSide::Top, "toolbar")
            .frame(default_blurry_frame())
            .show(ctx, |ui| {
                self.topbar_ui(ui);
            })
            .response
            .rect;
        if let Ok(mut blur_regions) = self.q_camera_blur.get_single_mut() {
            let scale_factor = ctx.options(|op| op.zoom_factor);
            let min = vec2(rect.min.x, rect.min.y) * scale_factor;
            let max = vec2(rect.max.x, rect.max.y) * scale_factor;
            blur_regions.blur(Rect::from_corners(min, max));
        }
    }

    fn topbar_ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.parent_selector(ui);
            ui.separator();
            self.current_date(ui);
            ui.with_layout(egui::Layout::right_to_left(Align::Max), |ui| {
                if self.state.topbar_expanded {
                    self.fps_display_history(ui);
                } else {
                    self.fps_display(ui);
                }
                ui.separator();
                self.draw_orbit_toggle(ui);
                ui.checkbox(&mut self.state.show_labels, "Show labels");
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
                            self.commands.entity(entity).queue(Reparent(e));
                        }
                    }
                }
            });
    }

    fn current_date(&mut self, ui: &mut Ui) {
        let r1 = ui.label(
            self.mjd
                .context()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        );
        let r2 = ui.label(self.mjd.context().to_string());

        if r1.double_clicked() || r2.double_clicked() {
            self.state.date_window_opened = true;
        }
    }

    fn speed_controls(&mut self, ui: &mut Ui) {
        let mut cur_speed = self.time.relative_speed();
        ui.with_layout(egui::Layout::left_to_right(Align::Max), |ui| {
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

    fn date_time_window(&mut self, ctx: &egui::Context) {
        let mut open = self.state.date_window_opened;
        egui::Window::new("Date/Time")
            .open(&mut open)
            .collapsible(false)
            .max_height(400.0)
            .frame(default_blurry_frame())
            .show_with_blur(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Current date: ");
                    ui.label(self.mjd.context().format("%Y-%m-%d %H:%M:%S").to_string());
                });
                if let Some(new_datetime) = datetime_edit_widget(ui, **self.mjd.context()) {
                    self.mjd.context_mut().set_from_datetime(new_datetime);
                }

                ui.with_layout(egui::Layout::left_to_right(Align::Max), |ui| {
                    if ui.button("Close").clicked() {
                        self.state.date_window_opened = false;
                    }
                });
            });
        self.state.date_window_opened = open;
    }

    fn bodies_on_screen(&mut self, ctx: &egui::Context) {
        containers::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                let Ok((cam_transform, camera)) = self.q_camera_transform.get_single() else {
                    return;
                };

                if ui
                    .allocate_rect(ui.clip_rect(), egui::Sense::click())
                    .double_clicked()
                {
                    self.commands.trigger(pan_orbit::events::RecenterCamera);
                }

                if !self.state.show_labels {
                    return;
                }

                let painter = ui.painter();
                for (planet_transform, name) in &self.q_planetary_bodies {
                    const CIRCLE_SIZE: f32 = 5.0;
                    const TEXT_POS: f32 = CIRCLE_SIZE + 3.0;

                    let distance = planet_transform
                        .translation()
                        .distance(cam_transform.translation());
                    let distance = Distance::from(distance as f64);
                    let name = name
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| "Unknown Body".to_string());
                    let Ok(viewport) =
                        camera.world_to_viewport(cam_transform, planet_transform.translation())
                    else {
                        continue;
                    };
                    let text = format!("{name}\n{distance:.1}");
                    let center = egui::pos2(viewport.x, viewport.y);
                    painter.circle_filled(center, CIRCLE_SIZE, egui::Color32::WHITE);
                    painter.text(
                        center + egui::vec2(TEXT_POS, TEXT_POS),
                        egui::Align2::LEFT_CENTER,
                        text,
                        FontId::proportional(11.0),
                        egui::Color32::WHITE,
                    );
                }

                for (transform, star, name) in &self.q_stars {
                    const CIRCLE_SIZE: f32 = 3.0;
                    const TEXT_POS: f32 = CIRCLE_SIZE + 3.0;

                    let Ok(viewport) =
                        camera.world_to_viewport(cam_transform, transform.translation())
                    else {
                        continue;
                    };
                    if star.relative_magnitude > 3.0 {
                        continue;
                    }
                    let distance = Distance {
                        value: star.distance_parsecs,
                        unit: DistanceUnit::Parsecs,
                    }
                    .renormalize();
                    let text = format!("{name}\n{distance:.1}");
                    let center = egui::pos2(viewport.x, viewport.y);
                    let color = {
                        let [r, g, b] = star.blackbody_color().to_u8_array_no_alpha();
                        egui::Color32::from_rgb(r, g, b)
                    };
                    painter.circle_filled(center, CIRCLE_SIZE, color);
                    painter.text(
                        center + egui::vec2(TEXT_POS, TEXT_POS),
                        egui::Align2::LEFT_CENTER,
                        text,
                        FontId::proportional(11.0),
                        color,
                    );
                }
            });
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

fn datetime_edit_widget(ui: &mut Ui, input: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let mut ret = None;
    ui.columns_const(|[left, right]| {
        let mut year = input.year();
        let mut month = input.month();
        let mut day = input.day();
        let mut hour = input.hour();
        let mut minute = input.minute();
        let mut second = input.second();
        match number_widget(left, right, &mut year, "Year", None) {
            NumberChanged::Changed => ret = input.with_year(year),
            NumberChanged::Next => ret = input.checked_add_months(Months::new(12)),
            NumberChanged::Prev => ret = input.checked_sub_months(Months::new(12)),
            _ => {}
        }
        match number_widget(left, right, &mut month, "Month", Some(1..=12)) {
            NumberChanged::Changed => ret = input.with_month(month),
            NumberChanged::Next => ret = input.checked_add_months(Months::new(1)),
            NumberChanged::Prev => ret = input.checked_sub_months(Months::new(1)),
            _ => {}
        }
        match number_widget(left, right, &mut day, "Day", Some(1..=31)) {
            NumberChanged::Changed => ret = input.with_day(day),
            NumberChanged::Next => ret = input.checked_add_days(Days::new(1)),
            NumberChanged::Prev => ret = input.checked_sub_days(Days::new(1)),
            _ => {}
        }
        match number_widget(left, right, &mut hour, "Hour", Some(0..=23)) {
            NumberChanged::Changed => ret = input.with_hour(hour),
            NumberChanged::Next => ret = input.checked_add_signed(TimeDelta::hours(1)),
            NumberChanged::Prev => ret = input.checked_sub_signed(TimeDelta::hours(1)),
            _ => {}
        }
        match number_widget(left, right, &mut minute, "Minute", Some(0..=59)) {
            NumberChanged::Changed => ret = input.with_minute(minute),
            NumberChanged::Next => ret = input.checked_add_signed(TimeDelta::minutes(1)),
            NumberChanged::Prev => ret = input.checked_sub_signed(TimeDelta::minutes(1)),
            _ => {}
        }
        match number_widget(left, right, &mut second, "Second", Some(0..=59)) {
            NumberChanged::Changed => ret = input.with_second(second),
            NumberChanged::Next => ret = input.checked_add_signed(TimeDelta::seconds(1)),
            NumberChanged::Prev => ret = input.checked_sub_signed(TimeDelta::seconds(1)),
            _ => {}
        }
    });
    ret
}

enum NumberChanged {
    Changed,
    Next,
    Prev,
    Unchanged,
}

fn number_widget<T: emath::Numeric>(
    left: &mut Ui,
    right: &mut Ui,
    value: &mut T,
    label: &str,
    range: Option<ops::RangeInclusive<T>>,
) -> NumberChanged {
    let mut ret = NumberChanged::Unchanged;
    let id = left.label(label).id;
    let widget = widgets::DragValue::new(value).update_while_editing(false);
    let widget = if let Some(range) = range {
        widget.range(range)
    } else {
        widget
    };
    right.horizontal(|ui| {
        if ui.button("<").clicked() {
            ret = NumberChanged::Prev;
        }
        if ui.add(widget).labelled_by(id).changed() {
            ret = NumberChanged::Changed;
        }
        if ui.button(">").clicked() {
            ret = NumberChanged::Next;
        }
    });
    ret
}

fn default_blurry_frame() -> egui::Frame {
    let default_frame = egui::Frame::default();
    let [r, g, b, _] = default_frame.fill.to_array();
    default_frame
        .fill(egui::Color32::from_rgba_unmultiplied(r, g, b, 128))
        .inner_margin(egui::vec2(8.0, 6.0))
}
