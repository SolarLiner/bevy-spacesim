mod math;
mod orbit;
mod solar_system;
mod space;

use bevy::prelude::*;
use bevy_editor_pls::controls;
use bevy_editor_pls::controls::EditorControls;
use bevy_editor_pls::editor::Editor;

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
            big_space::BigSpacePlugin::<space::PrecisionBase>::new(true),
            big_space::debug::FloatingOriginDebugPlugin::<i64>::default(),
            big_space::camera::CameraControllerPlugin::<space::PrecisionBase>::default(),
            bevy_editor_pls::EditorPlugin::default(),
        ))
        .add_plugins((solar_system::SolarSystemPlugin, orbit::OrbitPlugin))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
        })
        .insert_resource(editor_controls())
        .add_systems(Startup, (setup, camera_setup))
        .add_systems(PostUpdate, (cursor_grab_system, update_sim_speed))
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DynamicSceneBundle {
        scene: asset_server.load("scenes/solar.system.yaml"),
        ..default()
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
