use bevy::prelude::*;

use crate::components::{PanOrbitSettings, PanOrbitState};
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use big_space::precision::GridPrecision;
use big_space::{GridCell, ReferenceFrame};
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use crate::events::RecenterCamera;

#[derive(Default)]
pub(crate) struct BlockedInputs {
    pointer: bool,
    keyboard: bool,
}

pub(crate) fn get_blocked_inputs(
    mut egui: bevy_egui::EguiContexts,
    q_window_entities: Query<Entity, With<Window>>,
) -> BlockedInputs {
    let mut ret = BlockedInputs::default();
    for window_entity in &q_window_entities {
        if let Some(ctx) = egui.try_ctx_for_entity_mut(window_entity) {
            ret.pointer |= ctx.wants_pointer_input();
            ret.keyboard |= ctx.wants_keyboard_input();
        }
    }
    ret
}

#[allow(clippy::type_complexity)]
pub(crate) fn pan_orbit_camera<Prec: GridPrecision>(
    In(blocked_inputs): In<BlockedInputs>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut evr_motion: EventReader<MouseMotion>,
    mut evr_scroll: EventReader<MouseWheel>,
    mut q_camera: Query<(
        &PanOrbitSettings,
        &mut PanOrbitState,
        &mut Transform,
        Option<&mut GridCell<Prec>>,
        Option<&Parent>,
    )>,
    q_parent_ref_frame: Query<Option<&ReferenceFrame<Prec>>>,
) {
    if blocked_inputs.pointer {
        return;
    }

    // First, accumulate the total amount of
    // mouse motion and scroll, from all pending events:
    let mut total_motion: Vec2 = evr_motion.read().map(|ev| ev.delta).sum();

    // Reverse Y (Bevy's Worldspace coordinate system is Y-Up,
    // but events are in window/ui coordinates, which are Y-Down)
    total_motion.y = -total_motion.y;

    let mut total_scroll_lines = Vec2::ZERO;
    let mut total_scroll_pixels = Vec2::ZERO;
    for ev in evr_scroll.read() {
        match ev.unit {
            MouseScrollUnit::Line => {
                total_scroll_lines.x += ev.x;
                total_scroll_lines.y -= ev.y;
            }
            MouseScrollUnit::Pixel => {
                total_scroll_pixels.x += ev.x;
                total_scroll_pixels.y -= ev.y;
            }
        }
    }

    for (settings, mut state, mut transform, grid_cell, parent) in &mut q_camera {
        // Check how much of each thing we need to apply.
        // Accumulate values from motion and scroll,
        // based on our configuration settings.

        let mut total_pan = Vec2::ZERO;
        let mut total_orbit = Vec2::ZERO;
        if mouse_buttons.pressed(MouseButton::Left)
            && !blocked_inputs.keyboard
            && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight))
        {
            total_pan -= total_motion * settings.pan_sensitivity;
        } else if mouse_buttons.pressed(MouseButton::Left) {
            total_orbit -= total_motion * settings.orbit_sensitivity;
        }

        let mut total_zoom = Vec2::ZERO;
        // Zoom on scroll wheel
        total_zoom -=
            total_scroll_lines * settings.scroll_line_sensitivity * settings.zoom_sensitivity;
        total_zoom -=
            total_scroll_pixels * settings.scroll_pixel_sensitivity * settings.zoom_sensitivity;

        // Upon starting a new orbit maneuver (key is just pressed),
        // check if we are starting it upside-down
        if mouse_buttons.just_pressed(MouseButton::Left) {
            state.upside_down = state.pitch < -FRAC_PI_2 || state.pitch > FRAC_PI_2;
        }

        // If we are upside down, reverse the X orbiting
        if state.upside_down {
            total_orbit.x = -total_orbit.x;
        }

        // Now we can actually do the things!

        let mut any = false;

        // To ZOOM, we need to multiply our radius.
        if total_zoom != Vec2::ZERO {
            any = true;
            // in order for zoom to feel intuitive,
            // everything needs to be exponential
            // (done via multiplication)
            // not linear
            // (done via addition)

            // so we compute the exponential of our
            // accumulated value and multiply by that
            state.radius *= (-total_zoom.y).exp();
        }

        // To ORBIT, we change our pitch and yaw values
        if total_orbit != Vec2::ZERO {
            any = true;
            state.yaw += total_orbit.x;
            state.pitch += total_orbit.y;
            // wrap around, to stay between +- 180 degrees
            if state.yaw > PI {
                state.yaw -= TAU; // 2 * PI
            }
            if state.yaw < -PI {
                state.yaw += TAU; // 2 * PI
            }
            if state.pitch > PI {
                state.pitch -= TAU; // 2 * PI
            }
            if state.pitch < -PI {
                state.pitch += TAU; // 2 * PI
            }
        }

        // To PAN, we can get the UP and RIGHT direction
        // vectors from the camera's transform, and use
        // them to move the center point. Multiply by the
        // radius to make the pan adapt to the current zoom.
        if total_pan != Vec2::ZERO {
            any = true;
            let radius = state.radius;
            state.center += transform.right() * total_pan.x * radius;
            state.center += transform.up() * total_pan.y * radius;
        }

        // Finally, compute the new camera transform.
        // (if we changed anything, or if the pan-orbit
        // controller was just added, and thus we are running
        // for the first time and need to initialize)
        if state.is_changed() || any || state.is_added() {
            let parent = parent.and_then(|parent| q_parent_ref_frame.get(**parent).ok().flatten());

            // YXZ Euler Rotation performs yaw/pitch/roll.
            transform.rotation = Quat::from_euler(EulerRot::YXZ, state.yaw, state.pitch, 0.0);
            // To position the camera, get the backward direction vector
            // and place the camera at the desired radius from the center.
            let position = state.center + transform.back() * state.radius;
            match (grid_cell, parent) {
                (Some(mut cell), Some(frame)) => {
                    let (new_cell, local_pos) = frame.imprecise_translation_to_grid(position);
                    *cell = new_cell;
                    transform.translation = local_pos;
                }
                _ => {
                    transform.translation = position;
                }
            }
        }
    }
}

pub fn recenter_camera(_: Trigger<RecenterCamera>, mut q_state: Query<&mut PanOrbitState>) {
    for mut state in &mut q_state {
        debug!("Recentering camera");
        state.center = Vec3::ZERO;
    }    
}