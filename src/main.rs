#![allow(unused)]
mod player;
mod components;

use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, window::{self, PresentMode, PrimaryWindow, WindowTheme}};
use components::{Velocity, Movable, SpriteSize, FromPlayer, Laser, Enemy, ExplosionToSpawn, Explosion, ExplosionTimer, Player, FromEnemy};
use player::PlayerPlugin;

// region:     --- Asset Constants

const PLAYER_SPRITE: &str = "player_a_01.png";
const PLAYER_SIZE: (f32, f32) = (144., 75.);

const SPRITE_SCALE: f32 = 0.5;

// endregion:  --- Asset Constants

// region:    --- Game Constants

const TIME_STEP: f32 = 1./60.;
const BASE_SPEED: f32 = 500.;

const ENEMY_MAX: u32 = 2;
// endregion: --- Game Constants

// region:     --- Resources
#[derive(Resource)]
pub struct WinSize {
	pub width: f32,
	pub height: f32
}

#[derive(Resource)]
struct GameTextures {
	player: Handle<Image>,
}

// endregion:  --- Resources

fn main() {
    App::new()
		.add_plugins((
			DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Rust Asteroids!".into(),
					name: Some("bevy.app".into()),
					resolution: (500., 300.).into(),
					present_mode: PresentMode::AutoVsync,
					// Tells wasm not to override default event handling, like F5, Ctrl+R etc.
					prevent_default_event_handling: false,
					window_theme: Some(WindowTheme::Dark),
					enabled_buttons: bevy::window::EnabledButtons {
						maximize: false,
						..Default::default()
					},
					resizable: false,
					// This will spawn an invisible window
					// The window will be made visible in the make_visible() system after 3 frames.
					// This is useful when you want to avoid the white window that shows up before the GPU is ready to render the app.
					visible: false,
					..default()
				}),
				..default()
			}),
			LogDiagnosticsPlugin::default(),
			FrameTimeDiagnosticsPlugin,
		))
		.add_plugins(PlayerPlugin)
		.add_systems(Startup, setup_system)
		.add_systems(Update, make_visible)
		.run();
}

fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut windows:  Query<&mut Window, With<PrimaryWindow>>,
) {
	// camera
	commands.spawn(Camera2dBundle::default());

	// capture window size
	let window = windows.get_single_mut().unwrap();
	let (win_w, win_h) = (window.width(), window.height());

	// add WinSize resource
	let win_size = WinSize { width: win_w, height: win_h };
	commands.insert_resource(win_size);

	// add GameTextures resource
	let game_textures = GameTextures { player: asset_server.load(PLAYER_SPRITE) };
	commands.insert_resource(game_textures);
}

fn make_visible(mut window: Query<&mut Window>, frames: Res<FrameCount>) {
    // The delay may be different for your app or system.
    if frames.0 == 3 {
        // At this point the gpu is ready to show the app so we can make the window visible.
        // Alternatively, you could toggle the visibility in Startup.
        // It will work, but it will have one white frame before it starts rendering
        window.single_mut().visible = true;
    }
}