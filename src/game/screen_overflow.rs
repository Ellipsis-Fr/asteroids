use super::{components::{self, EntityType}, GameTextures, WinSize};

use bevy::{ecs::query::QueryEntityError, prelude::*};
use bevy_rapier2d::prelude::{ ActiveEvents, Collider, CollisionEvent, ContactForceEvent, RigidBody, Velocity };
use components::{Direction, Enemy, Explosion, ExplosionTimer, ExplosionToSpawn, Fake, FakeEntities, FromEnemy, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, MeteorLevel, Player, RocketDragTimer};

const MARGIN: f32 = 10.;

pub fn correction_screen_overflow_small_entities(win_size: &Res<WinSize>, mut query: Query<&mut Transform, (Without<Fake>, Without<FakeEntities>)>) {
	for mut transform in query.iter_mut() {
        let translation = &mut transform.translation;

		let new_position = |actual_position: f32, screen_limit: f32| -> f32 {
			if actual_position > screen_limit {
				-screen_limit
			} else if actual_position < -screen_limit {
				screen_limit
			} else {
				actual_position
			}
		};

		translation.x = new_position(translation.x, win_size.width / 2. + MARGIN);
		translation.y = new_position(translation.y, win_size.height / 2. + MARGIN);
    }
}

pub fn correction_screen_overflow_large_entities(
	mut commands: Commands,
	win_size: Res<WinSize>,
	mut large_movable_entities_query: Query<(Entity, &mut Transform, &Collider, &mut FakeEntities), Without<Fake>>,
	large_movable_entities_with_velocity_query: Query<&Velocity, (With<FakeEntities>, Without<Fake>)>,
	game_textures: Res<GameTextures>,
	query_player: Query<&Player>,
	query_enemy: Query<&Enemy>,
	query_meteor: Query<&Meteor>
) {
    let screen_limits = (win_size.x_axys_limit, win_size.y_axys_limit);
	
	for (entity, mut transform, collider, mut fake_entities) in large_movable_entities_query.iter_mut() {
		let (x, y, z, radian_angle) = extract_xyzw(&transform);
		let (texture, component) = get_entity_texture_and_type(entity, &game_textures, &query_player, &query_enemy, &query_meteor);

		let velocity_result = large_movable_entities_with_velocity_query.get(entity);

		let bundle_dimensions = get_bundle_dimensions(collider, x, y, radian_angle, transform.rotation.clone());

		correct_screen_overflow_large_entity(&mut commands, screen_limits, &mut transform, collider, &mut fake_entities, x, y, z, texture, component, velocity_result, bundle_dimensions);
    }
}

fn correct_screen_overflow_large_entity(
	commands: &mut Commands,
	screen_limits: ((f32, f32), (f32, f32)),
	mut transform: &mut Transform,
	collider: &Collider,
	mut fake_entities: &mut FakeEntities,
	x: f32, y: f32, z: f32,
	texture: Handle<Image>,
	component: EntityType,
	velocity_result: Result<&Velocity, QueryEntityError>,
	bundle_dimensions: (f32, f32, f32, f32, f32, f32, f32, f32)
) {

	let ((screen_left_limit, screen_right_limit), (screen_bottom_limit, screen_top_limit)) = (screen_limits.0, screen_limits.1);
	
	let (
		entity_left_limit,
		entity_right_limit,
		entity_left_lenght,
		entity_right_lenght,
		entity_bottom_limit,
		entity_top_limit,
		entity_bottom_lenght,
		entity_top_lenght
	) = bundle_dimensions;
	
	let x_third_duplication = correct_or_duplicate_entity(
		commands,
		transform,
		collider,
		fake_entities,
		&texture,
		&component,
		velocity_result,
		"x",
		screen_right_limit, screen_left_limit, entity_right_limit, entity_left_limit, entity_right_lenght, entity_left_lenght
	);

	let mut y_third_duplication = correct_or_duplicate_entity(
		commands,
		transform,
		collider,
		fake_entities,
		&texture,
		&component,
		velocity_result,
		"y",
		screen_top_limit, screen_bottom_limit, entity_top_limit, entity_bottom_limit, entity_top_lenght, entity_bottom_lenght
	);

	match (x_third_duplication, y_third_duplication) {
		(Some(x), Some(y)) => {
			let translation = Vec3::new(x, y, transform.translation.z);
			spawn_duplicate_entity(commands, transform, collider, fake_entities, &texture, &component, velocity_result, translation);
		},
		(None, None) => fake_entities.0 = vec![],
		_ => () 
	}
}

fn extract_xyzw(transform: &Transform) -> (f32, f32, f32, f32) {
	let Vec3 {x, y, z} = transform.translation;
	let pos_or_neg_rotation = if transform.rotation.w > 0.0 { 1. } else { -1. };
	let radian_angle = transform.rotation.z.asin() * 2. * pos_or_neg_rotation;
	(x, y, z, radian_angle)
}

fn get_entity_texture_and_type(
	entity: Entity,
	game_textures: &Res<GameTextures>,
	query_player: &Query<&Player>,
	query_enemy: &Query<&Enemy>,
	query_meteor: &Query<&Meteor>
) -> (Handle<Image>, EntityType) {
	let (texture, component) = if let Ok(_) = query_player.get(entity) {
			(game_textures.player.clone(), EntityType::Player(Player))
		} else if let Ok(enemy) = query_enemy.get(entity) {
			(game_textures.enemy.clone(), EntityType::Enemy(enemy.clone()))
		} else if let Ok(_) = query_meteor.get(entity) {
			(game_textures.meteor.clone(), EntityType::Meteor(Meteor))
		} else {
			panic!("Unknow Entity able to cross the screen")
		};
	(texture, component)
}

fn correct_or_duplicate_entity(
	commands: &mut Commands,
	mut transform: &mut Transform,
	collider: &Collider,
	fake_entities: &mut FakeEntities,
	texture: &Handle<Image>,
	component: &EntityType,
	velocity_result: Result<&Velocity, QueryEntityError>,
	axis: &str,
	screen_limit_a: f32, screen_limit_b: f32, entity_limit_a: f32, entity_limit_b: f32, entity_lenght_a: f32, entity_lenght_b: f32
) -> Option<f32> {
	
	if let Some((new_position, needs_duplication)) = check_overflow_coordinate(screen_limit_a + MARGIN, screen_limit_b - MARGIN, entity_limit_a, entity_limit_b, entity_lenght_a, entity_lenght_b) {
		if !needs_duplication {
			
			match axis {
				"x" => transform.translation.x = new_position,
				"y" => transform.translation.y = new_position,
				_ => panic!("Invalid axys")
			}
			
			None
		} else {
			let mut translation = transform.translation;

			match axis {
				"x" => translation.x = new_position,
				"y" => translation.y = new_position,
				_ => panic!("Invalid axys")
			} 
	
			spawn_duplicate_entity(commands, transform, collider, fake_entities, texture, component, velocity_result, translation);
			
			Some(new_position)
		}
	} else {
		None
	}
}

fn spawn_duplicate_entity(
	commands: &mut Commands,
	transform: &mut Transform,
	collider: &Collider,
	fake_entities: &mut FakeEntities,
	texture: &Handle<Image>,
	component: &EntityType,
	velocity_result: Result<&Velocity, QueryEntityError>,
	translation: Vec3
) {
	let fake_entity = commands
		.spawn(SpriteBundle {
			texture: texture.clone(),
			transform: Transform {
				translation,
				rotation: transform.rotation,
				scale: transform.scale
			},
			..Default::default()
		})
		.insert(collider.clone())
		.insert(RigidBody::Dynamic)
		.insert(ActiveEvents::CONTACT_FORCE_EVENTS)
		.insert(Fake)
		.id();

	match component.clone() {
		EntityType::Player(player) => {
			commands.entity(fake_entity).insert(player);
		},
		EntityType::Enemy(enemy) => {
			commands.entity(fake_entity).insert(enemy);
		}
		EntityType::Meteor(meteor) => {
			commands.entity(fake_entity).insert(meteor);
		},
		_ => panic!()
	}

	if let Ok(velocity) = velocity_result {
		commands.entity(fake_entity).insert(velocity.clone());
	}

	fake_entities.0.push(fake_entity);
}

fn get_bundle_dimensions(collider: &Collider, x_position: f32, y_position: f32, radian_angle: f32, rotation: Quat) -> (f32, f32, f32, f32, f32, f32, f32, f32) {
	match true {
		_ if collider.as_ball().is_some() => {
			let radius = collider.as_ball().unwrap().radius();
			let (left_limit, right_limit) = (-1. * radius + x_position, 1. * radius + x_position);
			let (bottom_limit, top_limit) = (-1. * radius + y_position, 1. * radius + y_position);

			(left_limit, right_limit, radius, radius, bottom_limit, top_limit, radius, radius)
		},
		_ if collider.as_cuboid().is_some() => {
			let half_extents_top_left_edges = collider.as_cuboid().unwrap().half_extents();
			let half_extents_bottom_left_edges = half_extents_top_left_edges * Vec2::new(1., -1.);

			let hyp = half_extents_top_left_edges.length();
			let theta_top_left_edges = half_extents_top_left_edges.y.atan2(half_extents_top_left_edges.x);
			let theta_bottom_left_edges = half_extents_bottom_left_edges.y.atan2(half_extents_bottom_left_edges.x);

			let new_theta_top_left_edges = theta_top_left_edges + radian_angle;
			let new_x_1 = hyp * new_theta_top_left_edges.cos();
			let new_y_1 = hyp * new_theta_top_left_edges.sin();

			let new_theta_bottom_left_edges = theta_bottom_left_edges + radian_angle;
			let new_x_2 = hyp * new_theta_bottom_left_edges.cos();
			let new_y_2 = hyp * new_theta_bottom_left_edges.sin();

			let new_x = if new_x_1.abs() > new_x_2.abs() { new_x_1.abs() } else { new_x_2.abs() };
			let new_y = if new_y_1.abs() > new_y_2.abs() { new_y_1.abs() } else { new_y_2.abs() };
			
			let (left_limit, right_limit) = (-1. * new_x + x_position, 1. * new_x + x_position);
			let (bottom_limit, top_limit) = (-1. * new_y + y_position, 1. * new_y + y_position);

			(left_limit, right_limit, new_x, new_x, bottom_limit, top_limit, new_y, new_y)
		},
		_ if collider.as_triangle().is_some() => {
			let triangle_view = collider.as_triangle().unwrap();
			let [a, b, c] = triangle_view.vertices();

			let center = Vec2::new(x_position, y_position);

			let point_rotated_around = | point_to_rotate: Vec3, center: Vec3, rotation: Quat | -> Vec2 {
				let mut transform= Transform::from_translation(point_to_rotate);
				transform.rotate_around(center, rotation);
				transform.translation.truncate()
			};

			let a_rotated = point_rotated_around((center + a).extend(0.), center.clone().extend(0.), rotation.clone());
			let b_rotated = point_rotated_around((center + b).extend(0.), center.clone().extend(0.), rotation.clone());
			let c_rotated = point_rotated_around((center + c).extend(0.), center.clone().extend(0.), rotation.clone());

			let x_values = vec![a_rotated.x, b_rotated.x, c_rotated.x];
			let y_values = vec![a_rotated.y, b_rotated.y, c_rotated.y];

			let x_min = x_values.iter().copied().reduce(f32::min).unwrap();
			let x_max = x_values.iter().copied().reduce(f32::max).unwrap();
			let y_min = y_values.iter().copied().reduce(f32::min).unwrap();
			let y_max = y_values.iter().copied().reduce(f32::max).unwrap();

			let (left_limit, right_limit) = (x_min, x_max);
			let (bottom_limit, top_limit) = (y_min, y_max);
			let semi_lenght_x_l = (x_position - left_limit).abs();
			let semi_lenght_x_r = (right_limit - x_position).abs();
			let semi_lenght_y_t = (top_limit - y_position).abs();
 			let semi_lenght_y_b = (y_position - bottom_limit).abs();
			(left_limit, right_limit, semi_lenght_x_l, semi_lenght_x_r, bottom_limit, top_limit, semi_lenght_y_b, semi_lenght_y_t)

		},
		_ if collider.as_capsule().is_some() => {
			let collider_view = collider.as_capsule().unwrap();
			let (center, half_height, radius) = (collider_view.center(), collider_view.half_height(), collider_view.radius());
			// dbg!(center);
			let (left_limit, right_limit) = (x_position - radius, x_position + radius);
			let (bottom_limit, top_limit) = (y_position - (half_height + radius), y_position + (half_height + radius));
			// dbg!((x_position, y_position, left_limit, right_limit, radius, radius, bottom_limit, top_limit, (half_height + radius), (half_height + radius)));
			(left_limit, right_limit, radius, radius, bottom_limit, top_limit, (half_height + radius), (half_height + radius))

			// panic!()
			// (left_limit, right_limit, x_half_extent, x_half_extent, bottom_limit, top_limit, y_half_extent, y_half_extent)

		},
		_ => panic!()
	}
}

/// screen_limit_a : positive screen limit (right or top)
/// screen_limit_b : negative screen limit (left or bottom)
/// entity_limit_a : positive entity limit (right or top)
/// entity_limit_b : negative entity limit (left or bottom)
/// entity_lenght_a : positive entity lenght (right or top)
/// entity_lenght_b : negative entity lenght (left or bottom)
fn check_overflow_coordinate(screen_limit_a: f32, screen_limit_b: f32, entity_limit_a: f32, entity_limit_b: f32, entity_lenght_a: f32, entity_lenght_b: f32) -> Option<(f32, bool)> {
	if entity_limit_a > screen_limit_a {
		let screen_overflow = entity_limit_a - screen_limit_a;
		if entity_limit_b > screen_limit_a {
			Some((screen_limit_b + entity_lenght_b, false))
		} else {	
			Some((screen_limit_b - entity_lenght_a + screen_overflow, true))
		}
	} else if entity_limit_b < screen_limit_b {
		let screen_overflow = entity_limit_b - screen_limit_b;
		if entity_limit_a < screen_limit_b {
			Some((screen_limit_a - entity_lenght_a, false))
		} else {
			Some((screen_limit_a + entity_lenght_b + screen_overflow, true))
		}
	} else {
		None
	}
}

fn check_wrapped_position(
    entity_1_position: Vec3,
    entity_2_position: Vec3,
    current_coord: f32,
    screen_limit_a: f32,
    screen_limit_b: f32,
    is_x_axis: bool,
) -> (f32, f32) {
    let new_coord = if current_coord < 0. {
        screen_limit_a + (current_coord - screen_limit_b)
    } else {
        screen_limit_b + (current_coord - screen_limit_a)
    };

    let test_position = if is_x_axis {
        Vec3::new(new_coord, entity_2_position.y, entity_2_position.z)
    } else {
        Vec3::new(entity_2_position.x, new_coord, entity_2_position.z)
    };

    let new_distance = entity_1_position.distance(test_position);
    (new_coord, new_distance)
}

pub fn get_nearest_position(win_size: &Res<WinSize>, entity_1_position: Vec3, entity_2_position: Vec3) -> Vec3 {
    let (screen_left_limit, screen_right_limit) = win_size.x_axys_limit;
    let (screen_bottom_limit, screen_top_limit) = win_size.y_axys_limit;

    let mut distance = entity_1_position.distance(entity_2_position);
    let mut nearest_entity_2_position = entity_2_position;

    // Check X-axis wrapping
    let (new_x, x_distance) = check_wrapped_position(
        entity_1_position,
        entity_2_position,
        entity_2_position.x,
        screen_right_limit,
        screen_left_limit,
        true,
    );
    if x_distance < distance {
        distance = x_distance;
        nearest_entity_2_position = Vec3::new(new_x, entity_2_position.y, entity_2_position.z);
    }

    // Check Y-axis wrapping
    let (new_y, y_distance) = check_wrapped_position(
        entity_1_position,
        entity_2_position,
        entity_2_position.y,
        screen_top_limit,
        screen_bottom_limit,
        false,
    );
    if y_distance < distance {
        distance = y_distance;
        nearest_entity_2_position = Vec3::new(entity_2_position.x, new_y, entity_2_position.z);
    }

    // Check diagonal wrapping
    let diagonal_distance = entity_1_position.distance(Vec3::new(new_x, new_y, entity_2_position.z));
    if diagonal_distance < distance {
        nearest_entity_2_position = Vec3::new(new_x, new_y, entity_2_position.z);
    }

    nearest_entity_2_position
}