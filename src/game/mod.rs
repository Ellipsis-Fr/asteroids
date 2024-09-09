#![allow(unused)]
mod player;
mod components;

use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, window::{self, PresentMode, PrimaryWindow, WindowTheme}};
use components::{Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, LaserTimer, Movable, Player, Rotation, SpriteSize, Velocity};
use player::PlayerPlugin;

// region:     --- Asset Constants

const PLAYER_SPRITE: &str = "spaceShips.png";
const PLAYER_SIZE: (f32, f32) = (136., 84.);

const LASER_SPRITE: &str = "laser.png";
const LASER_SIZE: (f32, f32) = (9., 54.);

const SPRITE_SCALE: f32 = 0.5;

// endregion:  --- Asset Constants

// region:    --- Game Constants

const MARGIN: f32 = 100.;

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
	laser: Handle<Image>,
}

// endregion:  --- Resources

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(PlayerPlugin)
        .add_systems(Startup, setup_system)
		.add_systems(Update, make_visible)
		.add_systems(Update, (rotate_player_system, movable_system, check_laser_timer_system));
    }
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
	let game_textures = GameTextures { 
		player: asset_server.load(PLAYER_SPRITE),
		laser: asset_server.load(LASER_SPRITE),
	 };
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

fn rotate_player_system(
	mut query: Query<(&mut Transform, &Rotation), With<Player>>
) {
	if let Ok((mut transform, rotation)) = query.get_single_mut() {
		transform.rotation = Quat::from_rotation_z(rotation.rotation_angle_degrees.to_radians());
	}
}

fn movable_system(
	win_size: Res<WinSize>,
	mut query: Query<(&Velocity, &mut Transform, &Movable)>
) {
    for (velocity, mut transform, movable) in query.iter_mut() {
		let translation = &mut transform.translation;
		translation.x += velocity.x * TIME_STEP * BASE_SPEED;
		translation.y += velocity.y * TIME_STEP * BASE_SPEED;
		correction_if_screen_overflow(&win_size, &mut translation.x, &mut translation.y);
    }
}

fn check_laser_timer_system(
	mut commands: Commands,
	time: Res<Time>,
	mut query: Query<(Entity, &mut LaserTimer)>
) {
    for (entity, mut laser_timer) in query.iter_mut() {
		laser_timer.0.tick(time.delta());
		if laser_timer.0.just_finished() {
			commands.entity(entity).despawn();
		}
    }
}


fn calculate_translation(rotation_angle_degrees: f32, acceleration: f32) -> (f32, f32) {
	let angle_radians = rotation_angle_degrees.to_radians();
	let mut x = angle_radians.sin() * acceleration * -1.;
	let mut y = angle_radians.cos() * acceleration;
	(x, y)
}

fn correction_if_screen_overflow(win_size: &Res<WinSize>, mut x: &mut f32, mut y: &mut f32) {
	let width = win_size.width / 2.;
	let height = win_size.height / 2.;
	
	if *x > width + MARGIN {
		*x = -width;
	} else if *x < -width - MARGIN {
		*x = width;
	}
	if *y > height + MARGIN {
		*y = -height;
	} else if *y < -height - MARGIN {
		*y = height;
	}
}