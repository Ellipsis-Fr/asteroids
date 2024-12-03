#![allow(unused)]
mod player;
mod meteor;
mod components;
mod wave;

use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, ecs::entity, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, sprite::MaterialMesh2dBundle, window::{self, PresentMode, PrimaryWindow, WindowTheme}};
use bevy_rapier2d::{plugin::RapierConfiguration, prelude::{ ColliderMassProperties, CollisionEvent, ContactForceEvent, ExternalForce, RigidBody, Velocity }};
use components::{Direction, Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, FromEnemy, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, MeteorLevel, Player, RocketDragTimer};
use player::PlayerPlugin;
use meteor::{MeteorDefinition, MeteorPlugin};
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

#[derive(Resource)]
struct DestroyedMeteors(pub Vec<(MeteorDefinition, Vec3)>);

#[derive(Resource)]
struct Fragments(pub Vec<Vec3>);

// endregion:  --- Resources

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
		.register_type::<MeteorLevel>()
        .add_plugins(PlayerPlugin)
        .add_plugins(MeteorPlugin)
        .add_systems(Startup, setup_system)
		.add_systems(PostStartup, init_wave_system)
		.add_systems(Update, make_visible)
		.add_systems(Update, (correction_screen_overflow_system, check_life_time_system, handle_fire_events_system));
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

	commands.insert_resource(DestroyedMeteors(Vec::new()));
	commands.insert_resource(Fragments(Vec::new()));

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

fn correction_screen_overflow_system(win_size: Res<WinSize>, mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        let translation = &mut transform.translation;

		let new_position = |p: f32, screen_limit: f32| -> f32 {
			if p > screen_limit {
				-screen_limit
			} else if p < -screen_limit {
				screen_limit
			} else {
				p
			}
		};

		translation.x = new_position(translation.x, win_size.width / 2. + MARGIN);
		translation.y = new_position(translation.y, win_size.height / 2. + MARGIN);
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

fn handle_fire_events_system(
	mut commands: Commands,
	mut fragments: ResMut<Fragments>,
	mut destroyed_meteors: ResMut<DestroyedMeteors>,
	mut collision_events: EventReader<CollisionEvent>,
	query_meteor: Query<(Entity, &MeteorLevel, &ColliderMassProperties, &Velocity, &Transform), With<Meteor>>,
	query_laser: Query<(Entity, &Velocity), With<Laser>>
) {
    let mut entities_whose_collision_event_is_processed = HashSet::new();

	'outer: for collision_event in collision_events.read() {
		
		let (entity_a, entity_b) = match get_entities_touched(collision_event, &mut entities_whose_collision_event_is_processed) {
			None => continue,
			Some((entity_a, entity_b)) => (entity_a, entity_b)
		};

		let mut laser_direction = None;
		for (entity_laser, velocity) in &query_laser {
			if entity_laser == entity_a || entity_laser == entity_b {
				let x = if velocity.linvel.x > 0. { 1. } else { -1. };
				let y = if velocity.linvel.y > 0. { 1. } else { -1. };
				laser_direction = Some(Vec2 {x, y});
			}
		}

		for (entity_meteor, meteor_level, mass, velocity, transform) in &query_meteor {
			if entity_meteor == entity_a || entity_meteor == entity_b {
				let meteor_velocity = apply_laser_direction_on_meteor(velocity, laser_direction.unwrap());
				handle_entity_destruction(&mut fragments, &mut destroyed_meteors, meteor_level, mass, meteor_velocity, transform);
				commands.entity(entity_a).despawn();
				commands.entity(entity_b).despawn();
				break 'outer;
			}
		}
    }
}

fn apply_laser_direction_on_meteor(velocity: &Velocity, laser_direction: Vec2) -> Vec2 {
	let direction = |meteor_direction, laser_direction| -> f32 {
		if meteor_direction > 0. {
			if laser_direction > 0. {
				meteor_direction
			} else {
				meteor_direction * -1.
			}
		} else {
			if laser_direction > 0. {
				meteor_direction * -1.
			} else {
				meteor_direction
			}
		}
	};

	Vec2 { x: direction(velocity.linvel.x, laser_direction.x), y: direction(velocity.linvel.y, laser_direction.y) }
}

fn get_entities_touched(collision_event: &CollisionEvent, entities_whose_collision_event_is_processed: &mut HashSet<Entity>) -> Option<(Entity, Entity)> {
	if let CollisionEvent::Started(entity_a, entity_b, _) = collision_event {
		if entities_whose_collision_event_is_processed.contains(entity_a) || entities_whose_collision_event_is_processed.contains(entity_b) {
			None
		} else {
			entities_whose_collision_event_is_processed.insert(entity_a.clone());
			entities_whose_collision_event_is_processed.insert(entity_b.clone());
			Some((entity_a.clone(), entity_b.clone()))
		}
	} else {
		None
	}
}

fn handle_entity_destruction(
	mut fragments: &mut ResMut<Fragments>,
	mut destroyed_meteors: &mut ResMut<DestroyedMeteors>,
	meteor_level: &MeteorLevel,
	mass: &ColliderMassProperties,
	velocity: Vec2,
	transform: &Transform
) {
	let entity_translation = transform.translation; 
	
	fragments.0.push(entity_translation.clone());

	if meteor_level.0 < 3 {
		destroyed_meteors.0.push((
			MeteorDefinition {
				weight: match mass {
						ColliderMassProperties::Mass(value) => *value,
						_ => panic!()
					},
				speed: [velocity.x, velocity.y],
				kind: 0,
				level: meteor_level.0
			},
			entity_translation.clone()
		));
	}
}