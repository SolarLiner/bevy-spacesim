mod math;
mod orbit;
mod solar_system;
mod space;

use crate::solar_system::{body, scene, sun};
use bevy::asset::LoadState;
use bevy::core_pipeline::bloom::BloomSettings;
use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::camera::Exposure;
use bevy::scene::SceneInstance;
use bevy_editor_pls::controls;
use bevy_editor_pls::controls::EditorControls;
use bevy_editor_pls::editor::Editor;
use big_space::camera::CameraController;
use big_space::{BigSpaceCommands, FloatingOrigin, GridCell, ReferenceFrame};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .build()
                .disable::<TransformPlugin>()
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..default()
                }),
            big_space::BigSpacePlugin::<space::Precision>::new(false),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
            big_space::camera::CameraControllerPlugin::<space::Precision>::default(),
            bevy_editor_pls::EditorPlugin::default(),
        ))
        .add_plugins((
            solar_system::SolarSystemPlugin,
            body::BodyPlugin,
            orbit::OrbitPlugin,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .insert_resource(editor_controls())
        .insert_state(SimState::Loading)
        .add_systems(OnEnter(SimState::Loading), load_scene)
        .add_systems(Update, transition_state.run_if(scene_ready))
        .add_systems(OnEnter(SimState::Running), (setup, camera_setup))
        .add_systems(
            PostUpdate,
            (cursor_grab_system, update_sim_speed).run_if(in_state(SimState::Running)),
        )
        .run();
}

fn editor_controls() -> EditorControls {
    let mut editor_controls = EditorControls::default_bindings();
    editor_controls.unbind(controls::Action::PlayPauseEditor);
    editor_controls.insert(
        controls::Action::PlayPauseEditor,
        controls::Binding {
            input: controls::UserInput::Chord(vec![
                controls::Button::Keyboard(KeyCode::Space),
                controls::Button::Keyboard(KeyCode::Space),
            ]),
            conditions: vec![
                controls::BindingCondition::EditorActive(false),
                controls::BindingCondition::InViewport(true),
            ],
        },
    );
    editor_controls
}

#[derive(Resource, Deref)]
struct LoadingScene(Handle<DynamicScene>);

fn load_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load("scenes/solar.system.yaml");
    commands.spawn_big_space(space::reference_frame(), |commands| {
        commands.insert((
            GridCell::<space::Precision>::default(),
            DynamicSceneBundle {
                scene: handle.clone(),
                ..default()
            },
        ));
    });
    commands.insert_resource(LoadingScene(handle));
}

fn transition_state(mut next_state: ResMut<NextState<SimState>>) {
    next_state.set(SimState::Running);
}

fn scene_ready(
    asset_server: Res<AssetServer>,
    loading_scene: Res<LoadingScene>,
    q: Query<(), With<SceneInstance>>,
) -> bool {
    asset_server
        .get_load_state(loading_scene.id())
        .is_some_and(|state| state == LoadState::Loaded)
        && !q.is_empty()
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, States)]
enum SimState {
    Loading,
    Running,
}

fn setup(
    mut commands: Commands,
    q_sun: Query<(Entity, &GridCell<space::Precision>, &Transform), With<scene::CameraTarget>>,
) {
    let (parent_entity, &cam_cell, &cam_transform) = q_sun.single();
    debug!("Camera target from scene at {parent_entity}");
    commands
        .entity(parent_entity)
        .remove::<scene::CameraTarget>()
        .with_children(|children| {
            children
                .spawn((
                    FloatingOrigin,
                    TransformBundle::from_transform(cam_transform),
                    VisibilityBundle::default(),
                    CameraController::default()
                        .with_speed_bounds([0.1, 1e35])
                        .with_smoothness(0.9, 0.9)
                        .with_speed(1.0),
                    cam_cell,
                ))
                .with_children(|children| {
                    children.spawn((
                        Camera3dBundle {
                            transform: Transform::from_xyz(0.0, 4.0, 22.0),
                            camera: Camera {
                                hdr: true,
                                ..default()
                            },
                            exposure: Exposure::SUNLIGHT,
                            ..default()
                        },
                        BloomSettings::default(),
                    ));
                });
        });
}

fn camera_setup(mut cam: ResMut<big_space::camera::CameraInput>) {
    cam.defaults_disabled = true;
}

fn cursor_grab_system(
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
    mut cam: ResMut<big_space::camera::CameraInput>,
    btn: Res<ButtonInput<MouseButton>>,
    key: Res<ButtonInput<KeyCode>>,
    editor: Option<Res<Editor>>,
) {
    let Some(mut window) = windows.get_single_mut().ok() else {
        return;
    };

    if editor.as_deref().is_some_and(|editor| editor.active()) {
        return;
    }

    if btn.just_pressed(MouseButton::Right) {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
        window.cursor.visible = false;
        // window.mode = WindowMode::BorderlessFullscreen;
        cam.defaults_disabled = false;
    }

    if key.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = bevy::window::CursorGrabMode::None;
        window.cursor.visible = true;
        // window.mode = WindowMode::Windowed;
        cam.defaults_disabled = true;
    }
}

fn update_sim_speed(mut time: ResMut<Time<Virtual>>, key: Res<ButtonInput<KeyCode>>) {
    if key.just_pressed(KeyCode::Space) {
        if time.was_paused() {
            info!("Unpause");
            time.unpause();
        } else {
            info!("Pause");
            time.pause();
        }
    }

    let mut changed = false;
    if key.just_pressed(KeyCode::KeyJ) {
        let new_speed = (time.relative_speed() / 10.).max(0.1);
        time.set_relative_speed(new_speed);
        changed = true;
    } else if key.just_pressed(KeyCode::KeyK) {
        let new_speed = (time.relative_speed() * 10.).min(1e4);
        time.set_relative_speed(new_speed);
        changed = true;
    }
    if changed {
        info!("Relative speed: {}x", time.relative_speed(),);
    }
}
