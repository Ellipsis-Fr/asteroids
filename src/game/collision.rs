use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

use super::{components::*, meteor::MeteorDefinition, DestroyedMeteors, Fragments};


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