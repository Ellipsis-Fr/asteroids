
use super::{components, WinSize};

use bevy::prelude::*;
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
	large_movable_entities_with_velocity_query: Query<&Velocity, (With<FakeEntities>, Without<Fake>)>
) {
    let (screen_left_limit, screen_right_limit) = win_size.x_axys_limit;
	let (screen_bottom_limit, screen_top_limit) = win_size.y_axys_limit;
	
	for (entity, mut transform, collider, mut fake_entities) in large_movable_entities_query.iter_mut() {
		let Vec3 {x, y, z} = transform.translation;
		let pos_or_neg_rotation = if transform.rotation.w > 0.0 { 1. } else { -1. };
		let radian_angle = transform.rotation.z.asin() * 2. * pos_or_neg_rotation;
		let velocity_result = large_movable_entities_with_velocity_query.get(entity);

		let (
			entity_left_limit,
			entity_right_limit,
			entity_left_lenght,
			entity_right_lenght,
			entity_bottom_limit,
			entity_top_limit,
			entity_bottom_lenght,
			entity_top_lenght
		) = get_bundle_dimensions(collider, x, y, radian_angle, transform.rotation.clone());

		let mut x_third_duplication = None;
		if let Some((x, needs_duplication)) = check_overflow_coordinate(screen_right_limit, screen_left_limit, entity_right_limit, entity_left_limit, entity_right_lenght, entity_left_lenght) {
			if !needs_duplication {
				transform.translation.x = x;
			} else {
				x_third_duplication = Some(x);
				
				let fake_entity = commands
					.spawn(collider.clone())
					.insert(RigidBody::Dynamic)
					.insert(ActiveEvents::CONTACT_FORCE_EVENTS)
					.insert(Fake)
					.insert(TransformBundle::from(Transform {
								translation: Vec3 { x, y, z },
								rotation: transform.rotation,
								scale: transform.scale
							}
					))
					.id();

				if let Ok(velocity) = velocity_result {
					commands.entity(fake_entity).insert(velocity.clone());
				}

				fake_entities.0.push(fake_entity);

			}
		}

		let mut y_third_duplication = None;
		if let Some((y, needs_duplication)) = check_overflow_coordinate(screen_top_limit, screen_bottom_limit, entity_top_limit, entity_bottom_limit, entity_top_lenght, entity_bottom_lenght) {
			if !needs_duplication {
				transform.translation.y = y;
			} else {
				y_third_duplication = Some(y);

				let fake_entity = commands
					.spawn(collider.clone())
					.insert(RigidBody::Dynamic)
					.insert(ActiveEvents::CONTACT_FORCE_EVENTS)
					.insert(Fake)
					.insert(TransformBundle::from(Transform {
						translation: Vec3 { x, y, z },
						rotation: transform.rotation,
						scale: transform.scale
					}))
					.id();

				if let Ok(velocity) = velocity_result {
					commands.entity(fake_entity).insert(velocity.clone());
				}

				fake_entities.0.push(fake_entity);
			}
		}

		if x_third_duplication.is_some() && y_third_duplication.is_some() {
			let fake_entity = commands
					.spawn(collider.clone())
					.insert(RigidBody::Dynamic)
					.insert(ActiveEvents::CONTACT_FORCE_EVENTS)
					.insert(Fake)
					.insert(TransformBundle::from(Transform {
						translation: Vec3 { x: x_third_duplication.unwrap(), y: y_third_duplication.unwrap(), z },
						rotation: transform.rotation,
						scale: transform.scale
					}
				))
				.id();

			if let Ok(velocity) = velocity_result {
				commands.entity(fake_entity).insert(velocity.clone());
			}

			fake_entities.0.push(fake_entity);
			
		} else if x_third_duplication.is_none() && y_third_duplication.is_none() {
			fake_entities.0 = vec![];
		}
    }
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
			// dbg!((data_save.0, data_save.0.to_degrees(), radian_angle, radian_angle.to_degrees(), new_theta_1, new_theta_1.to_degrees()));
			// dbg!((x_position, y_position, left_limit, right_limit, bottom_limit, top_limit));
			// dbg!((new_x, new_x_1, new_x_2, new_y, new_y_1, new_y_2));
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