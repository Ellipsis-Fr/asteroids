use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

use super::{components::*, meteor::MeteorDefinition, Fragments};
use super::events::*;


pub fn handle_contact_from_duplicated_entities(
    mut contact_force_events: EventReader<ContactForceEvent>,
	fake_uncontrollable_entities_query: Query<(Entity, &Velocity), With<Fake>>,
	fake_controllable_entities_query: Query<Entity, (With<Fake>, Without<Velocity>)>,
	mut original_uncontrollable_entities_query: Query<(&mut FakeEntities, &mut Velocity), Without<Fake>>,
	mut original_controllable_entities_query: Query<(&mut FakeEntities, &mut KinematicCharacterController), (Without<Fake>, Without<Velocity>)>,
    query: Query<Entity, Or<(With<Laser>, With<RocketFire>, With<Spark>)>>
) {
    for contact_force_event in contact_force_events.read() {
        // println!("Received contact force event: {:?}", contact_force_event);

		let entity_from_collider1 = contact_force_event.collider1;
		let entity_from_collider2 = contact_force_event.collider2;

        if query.get(entity_from_collider1).is_ok() || query.get(entity_from_collider2).is_ok() {
            continue;
        }

        adapt_new_movement_on_original_entity_from_its_fake_entities_collisioned(
            entity_from_collider1,
            &fake_uncontrollable_entities_query,
            &fake_controllable_entities_query,
            &mut original_uncontrollable_entities_query,
            &mut original_controllable_entities_query
        );

        adapt_new_movement_on_original_entity_from_its_fake_entities_collisioned(
            entity_from_collider2,
            &fake_uncontrollable_entities_query,
            &fake_controllable_entities_query,
            &mut original_uncontrollable_entities_query,
            &mut original_controllable_entities_query
        );
	}
}

fn adapt_new_movement_on_original_entity_from_its_fake_entities_collisioned(
    entity_from_collider: Entity,
    fake_uncontrollable_entities_query: &Query<(Entity, &Velocity), With<Fake>>,
	fake_controllable_entities_query: &Query<Entity, (With<Fake>, Without<Velocity>)>,
	mut original_uncontrollable_entities_query: &mut Query<(&mut FakeEntities, &mut Velocity), Without<Fake>>,
	mut original_controllable_entities_query: &mut Query<(&mut FakeEntities, &mut KinematicCharacterController), (Without<Fake>, Without<Velocity>)>
) {
    if let Ok((entity_fake, velocity_fake)) = fake_uncontrollable_entities_query.get(entity_from_collider) {
        for (fake_entities, mut velocity) in original_uncontrollable_entities_query.iter_mut() {
            if fake_entities.0.contains(&entity_from_collider) {
                *velocity = velocity_fake.clone();
            }
        }
    } else if let Ok(_) = fake_controllable_entities_query.get(entity_from_collider) {
        for (fake_entities, mut kinematic_character_controller) in original_controllable_entities_query.iter_mut() {
            if fake_entities.0.contains(&entity_from_collider) {
                // todo
            }
        }
    }
}

pub fn handle_fire_events(
    mut commands: Commands,
	mut fragments: ResMut<Fragments>,
	mut destroyed_meteors: EventWriter<MeteorDestructionEvent>,
	mut collision_events: EventReader<CollisionEvent>,
	query_meteor: Query<(Entity, &Velocity, &Transform), With<Meteor>>,
	query_meteor_original: Query<(Entity, &FakeEntities, &MeteorLevel, &ColliderMassProperties), With<Meteor>>,
	query_meteor_fake: Query<Entity, (With<Meteor>, With<Fake>)>,
	query_laser: Query<(Entity, &Velocity), With<Laser>>
) {
    let mut entities_whose_collision_event_is_processed = HashSet::new();

	'outer: for collision_event in collision_events.read() {
		
		let (laser_entity, meteor_entity) = match get_entities_touched(collision_event, &query_meteor, &query_laser, &mut entities_whose_collision_event_is_processed) {
			None => continue,
			Some((laser_entity, meteor_entity)) => (laser_entity, meteor_entity)
		};

		let laser_direction = get_laser_direction(&query_laser, laser_entity);
		commands.entity(laser_entity).despawn();

		if let Ok(((_, velocity, transform))) = query_meteor.get(meteor_entity) {
			
			if let Ok(_) = query_meteor_fake.get(meteor_entity) {
				for (original_meteor_entity, fake_entities, meteor_level, mass) in query_meteor_original.iter() {
					if fake_entities.0.contains(&meteor_entity) {
						let meteor_velocity = apply_laser_direction_on_meteor(velocity, laser_direction);
						handle_entity_destruction(&mut fragments, &mut destroyed_meteors, meteor_level, mass, meteor_velocity, transform);
						commands.entity(original_meteor_entity).despawn();
					}
				}
			} else if let Ok(((_, _, meteor_level, mass))) = query_meteor_original.get(meteor_entity) {
				let meteor_velocity = apply_laser_direction_on_meteor(velocity, laser_direction);
				handle_entity_destruction(&mut fragments, &mut destroyed_meteors, meteor_level, mass, meteor_velocity, transform);
				commands.entity(meteor_entity).despawn();
			};
		}
    }
}

fn get_entities_touched(
	collision_event: &CollisionEvent,
	query_meteor: &Query<(Entity, &Velocity, &Transform), With<Meteor>>,
	query_laser: &Query<(Entity, &Velocity), With<Laser>>,
	entities_whose_collision_event_is_processed: &mut HashSet<Entity>
) -> Option<(Entity, Entity)> {
	if let CollisionEvent::Started(entity_a, entity_b, _) = collision_event {
		if entities_whose_collision_event_is_processed.contains(entity_a) || entities_whose_collision_event_is_processed.contains(entity_b) {
			None
		} else {
			entities_whose_collision_event_is_processed.insert(entity_a.clone());
			entities_whose_collision_event_is_processed.insert(entity_b.clone());

			let (laser_entity, meteor_entity) = identify_entities(query_meteor, query_laser, entity_a, entity_b);

			match (laser_entity, meteor_entity) {
				(Some(laser), Some(meteor)) => Some((laser, meteor)),
				_ => None
			}
		}
	} else {
		None
	}
}

fn identify_entities(
	query_meteor: &Query<(Entity, &Velocity, &Transform), With<Meteor>>,
	query_laser: &Query<(Entity, &Velocity), With<Laser>>,
	entity_a: &Entity,
	entity_b: &Entity
) -> (Option<Entity>, Option<Entity>) {
	let laser_entity = match query_laser.get(*entity_a) {
		Ok((entity, _)) => Some(entity),
		Err(_) => {
			match query_laser.get(*entity_b) {
				Ok((entity, _)) => Some(entity),
				Err(_) => None
			}
		}
	};

	let meteor_entity = match query_meteor.get(*entity_a) {
		Ok((entity, _, _)) => Some(entity),
		Err(_) => {
			match query_meteor.get(*entity_b) {
				Ok((entity, _, _)) => Some(entity),
				Err(_) => None
			}
		}
	};

	(laser_entity, meteor_entity)
}

fn get_laser_direction(query_laser: &Query<'_, '_, (Entity, &Velocity), With<Laser>>, laser_entity: Entity) -> Vec2 {
	match query_laser.get(laser_entity) {
		Ok((_, velocity)) => {
			let x = if velocity.linvel.x > 0. { 1. } else { -1. };
			let y = if velocity.linvel.y > 0. { 1. } else { -1. };
			Vec2 {x, y}
		},
		Err(e) => panic!("{:?}", e)
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

fn handle_entity_destruction(
	mut fragments: &mut ResMut<Fragments>,
	mut destroyed_meteors: &mut EventWriter<MeteorDestructionEvent>,
	meteor_level: &MeteorLevel,
	mass: &ColliderMassProperties,
	velocity: Vec2,
	transform: &Transform
) {
	let entity_translation = transform.translation; 
	
	fragments.0.push(entity_translation.clone());

	if meteor_level.0 < 3 {
		destroyed_meteors.send(MeteorDestructionEvent(
			(
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
			)
		));
	}
}