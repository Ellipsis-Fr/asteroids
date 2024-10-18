#![allow(unused)]
mod player;
mod meteor;
mod components;
mod wave;

use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle, window::{self, PresentMode, PrimaryWindow, WindowTheme}};
use components::{Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, LaserTimer, LifeTime, Movable, Player, RocketDragTimer, Direction, SpriteSize, Velocity};
use player::PlayerPlugin;
use meteor::MeteorPlugin;
use wave::Wave;



// region:     --- Asset Constants

const PLAYER_SPRITE: &str = "spaceShips.png";
const PLAYER_SIZE: (f32, f32) = (136., 84.);

const LASER_SPRITE: &str = "laser.png";
const LASER_SIZE: (f32, f32) = (9., 54.);

const ROCKET_FIRE_SPRITE: &str = "rocket_fire.png";
const ROCKET_FIRE_SIZE: (f32, f32) = (2000., 2000.);

const METEOR_SPRITE: &str = "meteore1.png";
const METEOR_SIZE: (f32, f32) = (147., 119.);

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
	rocket_fire: Handle<Image>,
	meteor: Handle<Image>,
}

// endregion:  --- Resources

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins(PlayerPlugin)
        .add_plugins(MeteorPlugin)
        .add_systems(Startup, setup_system)
		.add_systems(PostStartup, init_wave_system)
		.add_systems(Update, make_visible)
		.add_systems(Update, (movable_system, correction_screen_overflow_system, check_life_time_system));
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
		rocket_fire: asset_server.load(ROCKET_FIRE_SPRITE),
		meteor: asset_server.load(METEOR_SPRITE)
	 };
	commands.insert_resource(game_textures);
}

fn init_wave_system(mut commands: Commands) {
	commands.insert_resource(Wave::new());
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

fn movable_system(win_size: Res<WinSize>, mut query: Query<(&Velocity, &mut Transform, &Movable)>
) {
    for (velocity, mut transform, movable) in query.iter_mut() {
		let translation = &mut transform.translation;
		translation.x += velocity.x * TIME_STEP * BASE_SPEED;
		translation.y += velocity.y * TIME_STEP * BASE_SPEED;
		// correction_if_screen_overflow(&win_size, &mut translation.x, &mut translation.y);
    }
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

fn correction_screen_overflow_system(win_size: Res<WinSize>, mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        let translation = &mut transform.translation;

        let width = win_size.width / 2.;
        let height = win_size.height / 2.;
        
        if translation.x > width + MARGIN {
            translation.x = -width;
        } else if translation.x < -width - MARGIN {
            translation.x = width;
        }
        if translation.y > height + MARGIN {
            translation.y = -height;
        } else if translation.y < -height - MARGIN {
            translation.y = height;
        }
    }
}

fn check_life_time_system(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &mut LifeTime)>) {
    for (entity, mut life_time) in query.iter_mut() {
		life_time.0.tick(time.delta());
		if life_time.0.just_finished() {
			commands.entity(entity).despawn();
		}
    }
}