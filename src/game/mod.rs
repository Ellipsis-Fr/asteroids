#![allow(unused)]
mod player;
mod enemy;
mod meteor;
mod fragment;
mod tech_details;
mod components;
mod events;
mod wave;
mod screen_overflow;
mod collision;

use std::env;
use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, ecs::entity, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle, window::{self, PresentMode, PrimaryWindow, WindowTheme}};
use bevy_rapier2d::{plugin::RapierConfiguration, prelude::{ Collider, ColliderMassProperties, CollisionEvent, ContactForceEvent, ExternalForce, KinematicCharacterController, RigidBody, Velocity }};
use collision::CollisionGroupConfig;
use components::{Direction, Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, Fake, FakeEntities, FromEnemy, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, MeteorLevel, Player, RocketDragTimer, RocketFire, Spark};
use events::{FragmentEvent, MeteorDestructionEvent};
use fragment::FragmentPlugin;
use player::PlayerPlugin;
use enemy::EnemyPlugin;
use tech_details::TechDetailsPlugin;
use meteor::{MeteorDefinition, MeteorPlugin};
use wave::Wave;



// region:     --- Asset Constants

const PLAYER_SPRITE: &str = "spaceShips.png";
const PLAYER_SIZE: (f32, f32) = (136., 84.);

const ENEMY_SPRITE: &str = "spaceShips.png";
const ENEMY_SIZE: (f32, f32) = (136., 84.);

const LASER_SPRITE: &str = "laser.png";
const LASER_SIZE: (f32, f32) = (9., 54.);

const ROCKET_FIRE_SPRITE: &str = "rocket_fire.png";
const ROCKET_FIRE_SIZE: (f32, f32) = (2000., 2000.);

const METEOR_SPRITE: &str = "meteore1.png";
const METEOR_SIZE: (f32, f32) = (147., 119.);

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
	pub height: f32,
	pub x_axys_limit: (f32, f32),
	pub y_axys_limit: (f32, f32),
}

impl WinSize {
	fn new(width: f32, height: f32) -> Self {
		let x_axys_limit = (-1. * width / 2., 1. * width / 2.);
		let y_axys_limit = (-1. * height / 2., 1. * height / 2.);

		Self { width, height, x_axys_limit, y_axys_limit }
	}
}

#[derive(Resource)]
struct GameTextures {
	player: Handle<Image>,
	enemy: Handle<Image>,
	laser: Handle<Image>,
	rocket_fire: Handle<Image>,
	meteor: Handle<Image>,
}

// endregion:  --- Resources

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
		.register_type::<MeteorLevel>()
		.add_event::<MeteorDestructionEvent>()
		.add_event::<FragmentEvent>()
        .add_plugins(PlayerPlugin)
		.add_plugins(EnemyPlugin)
        .add_plugins(MeteorPlugin)
        .add_plugins(FragmentPlugin)
		.add_plugins(TechDetailsPlugin(env::var("ACTIVE_TECH_DETAIL").unwrap_or("false".to_string()) == "true"))
        .add_systems(Startup, setup_system)
		.add_systems(PostStartup, init_wave_system)
		.add_systems(Update, make_visible)
		.add_systems(Update, (correction_screen_overflow_system, check_life_time_system))
		.add_systems(Last, (handle_fire_events_system, handle_contact_from_duplicated_entities_system).chain())
		.add_systems(First, remove_fake_entities_system);
    }
}

fn setup_system(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	mut windows:  Query<&mut Window, With<PrimaryWindow>>,
	mut rapier_configuration: ResMut<RapierConfiguration>
) {
	// camera
	commands.spawn(Camera2dBundle::default());

	// capture window size
	let window = windows.get_single_mut().unwrap();
	let (win_w, win_h) = (window.width(), window.height());

	// add WinSize resource
	let win_size = WinSize::new(win_w, win_h);
	commands.insert_resource(win_size);

	// add GameTextures resource
	let game_textures = GameTextures { 
		player: asset_server.load(PLAYER_SPRITE),
		enemy: asset_server.load(ENEMY_SPRITE),
		laser: asset_server.load(LASER_SPRITE),
		rocket_fire: asset_server.load(ROCKET_FIRE_SPRITE),
		meteor: asset_server.load(METEOR_SPRITE)
	 };
	commands.insert_resource(game_textures);

	commands.insert_resource(CollisionGroupConfig::default());

	// cancel gravity effect
    rapier_configuration.gravity = Vec2::new(0., 0.);
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

fn correction_screen_overflow_system(
	mut commands: Commands,
	win_size: Res<WinSize>,
	mut small_movable_entities_query: Query<&mut Transform, (Without<Fake>, Without<FakeEntities>)>,
	mut large_movable_entities_query: Query<(Entity, &mut Transform, &Collider, &mut FakeEntities), Without<Fake>>,
	large_movable_entities_with_velocity_query: Query<&Velocity, (With<FakeEntities>, Without<Fake>)>,
	game_textures: Res<GameTextures>,
	query_player: Query<&Player>,
	query_enemy: Query<&Enemy>,
	query_meteor: Query<&Meteor>
) {
    screen_overflow::correction_screen_overflow_small_entities(&win_size, small_movable_entities_query);

	screen_overflow::correction_screen_overflow_large_entities(
		commands,
		win_size,
		large_movable_entities_query,
		large_movable_entities_with_velocity_query,
		game_textures,
		query_player,
		query_enemy,
		query_meteor
	);
}

fn check_life_time_system(mut commands: Commands, time: Res<Time>, mut query: Query<(Entity, &mut LifeTime)>) {
    for (entity, mut life_time) in query.iter_mut() {
		life_time.0.tick(time.delta());
		if life_time.0.just_finished() {
			commands.entity(entity).despawn();
		}
    }
}

fn handle_contact_from_duplicated_entities_system(
	mut contact_force_events: EventReader<ContactForceEvent>,
	fake_uncontrollable_entities_query: Query<(Entity, &Velocity), With<Fake>>,
	fake_controllable_entities_query: Query<(Entity), (With<Fake>, Without<Velocity>)>,
	mut original_uncontrollable_entities_query: Query<(&mut FakeEntities, &mut Velocity), Without<Fake>>,
	mut original_controllable_entities_query: Query<(&mut FakeEntities, &mut KinematicCharacterController), (Without<Fake>, Without<Velocity>)>,
	query: Query<Entity, Or<(With<Laser>, With<RocketFire>, With<Spark>)>>
) {
	collision::handle_contact_from_duplicated_entities(
		contact_force_events,
		fake_uncontrollable_entities_query,
		fake_controllable_entities_query,
		original_uncontrollable_entities_query,
		original_controllable_entities_query,
		query
	);
}

fn handle_fire_events_system(
	mut commands: Commands,
	mut fragment_event: EventWriter<FragmentEvent>,
	mut destroyed_meteors_event: EventWriter<MeteorDestructionEvent>,
	mut collision_events: EventReader<CollisionEvent>,
	query_meteor: Query<(Entity, &Velocity, &Transform), With<Meteor>>,
	query_meteor_original: Query<(Entity, &FakeEntities, &MeteorLevel, &ColliderMassProperties), With<Meteor>>,
	query_meteor_fake: Query<Entity, (With<Meteor>, With<Fake>)>,
	query_laser: Query<(Entity, &Velocity), With<Laser>>
) {
    collision::handle_fire_events(commands, fragment_event, destroyed_meteors_event, collision_events, query_meteor, query_meteor_original, query_meteor_fake, query_laser);
}


fn remove_fake_entities_system(mut commands: Commands, query_fake_entities: Query<Entity, With<Fake>>) {
	for entity in query_fake_entities.iter() {
		commands.entity(entity).despawn();
	}
}

// Kept just for check
// fn test_next_position_system(time: Res<Time>, query_projectiles: Query<(Entity, &Transform, &Velocity, &Collider)>) {
// 	for (projectile_entity, projectile_transform, projectile_velocity, projectile_collider) in query_projectiles.iter() {
// 		let projectile_actual_position = projectile_transform.translation.truncate();
// 		let projectile_futur_position = projectile_transform.translation + projectile_velocity.linvel.extend(0.) * time.delta_seconds();

// 		dbg!((time.delta_seconds(), projectile_actual_position, projectile_futur_position.truncate()));
		
// 	}
// }