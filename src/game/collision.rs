use bevy::math::NormedVectorSpace;
use bevy::{prelude::*, utils::HashSet};
use bevy_rapier2d::prelude::*;

use super::{components::*, meteor::MeteorDefinition};
use super::events::*;


pub const PLAYER_GROUP: Group = Group::GROUP_1;     // 0b0001
pub const ENEMY_GROUP: Group = Group::GROUP_2;      // 0b0010
pub const PROJECTILE_GROUP: Group = Group::GROUP_3; // 0b0100
pub const METEOR_GROUP: Group = Group::GROUP_4;       // 0b1000

#[derive(Resource)]
pub struct CollisionGroupConfig {
    pub player: CollisionGroups,
    pub enemy: CollisionGroups,
    pub enemy_sensor: CollisionGroups,
    pub projectile: CollisionGroups,
}

impl Default for CollisionGroupConfig {
    fn default() -> Self {
        Self {
            player: CollisionGroups::new(
                PLAYER_GROUP,
                ENEMY_GROUP | METEOR_GROUP | PROJECTILE_GROUP,
            ),
            enemy: CollisionGroups::new(
                ENEMY_GROUP,
                PLAYER_GROUP | METEOR_GROUP | PROJECTILE_GROUP,
            ),
            enemy_sensor: CollisionGroups::new(
                ENEMY_GROUP,
                PLAYER_GROUP,
            ),
            projectile: CollisionGroups::new(
                PROJECTILE_GROUP,
                PLAYER_GROUP | ENEMY_GROUP | METEOR_GROUP,
            ),
        }
    }
}

/// This function handle contacts on duplicated entities to report it on the original entity
/// 
/// The contacts come from 'ContactForceEvent' (rapier struct using Bevy Event<T> Trait), added to duplicated entities.
/// Theses events just to be add to one of the two entities in contact to trigger an Event
/// 
/// # Arguments
/// 
/// * 'contact_force_events' : Event triggered by contacts betwenn two entities having a RigidBody and with at least one of them having component 'ActiveEvents::CONTACT_FORCE_EVENTS'
/// * 'fake_uncontrollable_entities_query' : query of entities duplicated not playable (meteor, enemie ship) necessary to get the result of the impact on the entity
/// * 'fake_controllable_entities_query' : query of entities duplicated playable (player) necessary to get the result of the impact on the entity
/// * 'original_uncontrollable_entities_query' : query of entities original not playable (meteor, enemie ship)
/// * 'original_controllable_entities_query' : query of entities original playable (player)
/// * 'entities_ignored_query' : query of entities to ignore
/// 
pub fn handle_contact_from_duplicated_entities(
    mut contact_force_events: EventReader<ContactForceEvent>,
	fake_uncontrollable_entities_query: Query<(Entity, &Velocity), With<Fake>>,
	fake_controllable_entities_query: Query<Entity, (With<Fake>, Without<Velocity>)>,
	mut original_uncontrollable_entities_query: Query<(&mut FakeEntities, &mut Velocity), Without<Fake>>,
	mut original_controllable_entities_query: Query<(&mut FakeEntities, &mut KinematicCharacterController), (Without<Fake>, Without<Velocity>)>,
    entities_ignored_query: Query<Entity, Or<(With<Laser>, With<RocketFire>, With<Spark>)>>
) {
    for contact_force_event in contact_force_events.read() {

		let entity_from_collider1 = contact_force_event.collider1;
		let entity_from_collider2 = contact_force_event.collider2;

        if entities_ignored_query.get(entity_from_collider1).is_ok() || entities_ignored_query.get(entity_from_collider2).is_ok() {
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

/// This function transfers the consequences of the contact of the duplicated entity to the original entity
/// 
/// # Arguments
/// 
/// * 'entity_from_collider' : Entity resulting from ContactForceEvent
/// * 'fake_uncontrollable_entities_query' : query of entities duplicated not playable (meteor, enemie ship) necessary to get the result of the impact on the entity
/// * 'fake_controllable_entities_query' : query of entities duplicated playable (player) necessary to get the result of the impact on the entity
/// * 'original_uncontrollable_entities_query' : query of entities original not playable (meteor, enemie ship)
/// * 'original_controllable_entities_query' : query of entities original playable (player)
/// 
/// # Explanation
/// 
/// To apply the force resulting from ContactForceEvent set on the dupplicated entity we have to find its original entity,
/// 
/// At first we search it in 'fake_..._entities_query' to get the new **Velocity** resulting from the contact
/// 
/// Then we look for its original entity in the 'original_..._entities_query' by passing on all existing original entities,
/// and for each one we look if their associated **fake_entities** contains this fake entity
/// 
/// If so, we apply on it the new velocity
/// 
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

/// This function handle the contact between Laser and Destructible components (Meteor, Ship)
/// 
/// # Arguments
/// 
/// * 'commands' : Bevy struct to handle bevy world
/// * 'fragment_event' : struct FragementEvent implement Event<T> Bevy's Trait to trigger fragment creation
/// * 'destroyed_meteors_event' : struct MeteorDestructionEvent implement Event<T> Bevy's Trait to trigger meteor children creation
/// * 'collision_events' : Event triggered by contacts betwenn two entities with at least one of them having component 'ActiveEvents::COLLISION_EVENTS'
/// * 'query_meteor' : query for all meteors (originals and dupplicates)
/// * 'query_meteor_original' : query of originals meteors to get their specific information
/// * 'query_meteor_fake' : query of duplicated meteors to check if the hit meteor is one of them
/// * 'query_laser' : query of laser entity to know the impact direction
/// 
pub fn handle_fire_events(
    mut commands: Commands,
	mut fragment_event: EventWriter<FragmentEvent>,
	mut destroyed_meteors_event: EventWriter<MeteorDestructionEvent>,
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

		if let Ok(((_, meteor_velocity, transform))) = query_meteor.get(meteor_entity) {
			
			if let Ok(_) = query_meteor_fake.get(meteor_entity) {
				for (original_meteor_entity, fake_entities, meteor_level, mass) in query_meteor_original.iter() {
					if fake_entities.0.contains(&meteor_entity) {
						apply_fire_event(&mut fragment_event, &mut destroyed_meteors_event, laser_direction, meteor_level, mass, meteor_velocity, transform);
						commands.entity(original_meteor_entity).despawn();
					}
				}
			} else if let Ok(((_, _, meteor_level, mass))) = query_meteor_original.get(meteor_entity) {
				apply_fire_event(&mut fragment_event, &mut destroyed_meteors_event, laser_direction, meteor_level, mass, meteor_velocity, transform);
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

fn apply_fire_event(
	fragment_event: &mut EventWriter<'_, FragmentEvent>,
	destroyed_meteors_event: &mut EventWriter<'_, MeteorDestructionEvent>,
	laser_direction: Vec2,
	meteor_level: &MeteorLevel,
	mass: &ColliderMassProperties,
	meteor_velocity: &Velocity,
	transform: &Transform
) {
	let meteor_velocity = apply_laser_direction_on_meteor(meteor_velocity, laser_direction);
	handle_entity_destruction(fragment_event, destroyed_meteors_event, meteor_level, mass, meteor_velocity, transform);
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
	mut fragment_event: &mut EventWriter<FragmentEvent>,
	mut destroyed_meteors_event: &mut EventWriter<MeteorDestructionEvent>,
	meteor_level: &MeteorLevel,
	mass: &ColliderMassProperties,
	velocity: Vec2,
	transform: &Transform
) {
	let entity_translation = transform.translation; 
	
	fragment_event.send(FragmentEvent(entity_translation.clone()));

	destroyed_meteors_event.send(MeteorDestructionEvent(
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

pub fn check_if_collide(collider_1: &Collider, position_1: Vec2, collider_2: &Collider, position_2: Vec2) -> bool {
	let get_circle_collider = | collider: &Collider | -> Collider {
		if collider.as_ball().is_none() {
			get_circle_collider_from_actual_collider(collider).unwrap()
		} else {
			collider.clone()
		}
	};

	let circle_collider_1 = get_circle_collider(collider_1);
	let circle_collider_2 = get_circle_collider(collider_2);

	(position_1 - position_2).norm() <= circle_collider_1.as_ball().unwrap().radius() + circle_collider_2.as_ball().unwrap().radius()
}

pub fn check_if_collide_after_moving(collider_1: &Collider, position_1: Vec3, direction_1: Vec3, collider_2: &Collider, position_2: Vec3, direction_2: Vec3, distance: f32) -> (bool, bool) {
	let future_position_1 = position_1 + direction_1 * distance;
	let collide_after_move_1 = check_if_collide(collider_1, future_position_1.truncate(), collider_2, position_2.truncate());

	let future_position_2 = position_2 + direction_2 * distance;
	let collide_after_move_2 = check_if_collide(collider_1, position_1.truncate(), collider_2, future_position_2.truncate());
	
	(collide_after_move_1, collide_after_move_2)
}

pub fn get_circle_collider_from_actual_collider(collider: &Collider) -> Option<Collider> {
    if let Some(cuboid) = collider.as_cuboid() {
        let half_extent = cuboid.half_extents();
        Some(Collider::ball(half_extent.length()))
    } else if let Some(capsule) = collider.as_capsule() {
		let (half_height, radius) = (capsule.half_height(), capsule.radius());
		Some(Collider::ball(half_height + radius))		
	} else if let Some(triangle) = collider.as_triangle() {
        todo!()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

	
    #[test]
    fn test_check_if_collide() {
        // Test case 1: Colliding circles
        let collider1 = Collider::ball(5.0);
        let position1 = Vec2::new(0.0, 0.0);
        let collider2 = Collider::ball(5.0);
        let position2 = Vec2::new(8.0, 0.0);
        
        assert!(check_if_collide(&collider1, position1, &collider2, position2));
        
        // Test case 2: Non-colliding circles
        let position2_far = Vec2::new(15.0, 0.0);
        assert!(!check_if_collide(&collider1, position1, &collider2, position2_far));
        
        // Test case 3: Exactly touching circles
        let position2_touching = Vec2::new(10.0, 0.0);
        assert!(check_if_collide(&collider1, position1, &collider2, position2_touching));
        
        // Test case 4: Different sized circles
        let collider1_small = Collider::ball(2.0);
        let collider2_large = Collider::ball(8.0);
        let position2_edge = Vec2::new(9.0, 0.0);
        
        assert!(check_if_collide(&collider1_small, position1, &collider2_large, position2_edge));
    }
    
    #[test]
    fn test_check_if_collide_after_moving() {
        let collider1 = Collider::ball(5.0);
        let position1 = Vec3::new(0.0, 0.0, 0.0);
        let direction1 = Vec3::new(1.0, 0.0, 0.0).normalize();
        
        let collider2 = Collider::ball(5.0);
        let position2 = Vec3::new(20.0, 0.0, 0.0);
        let direction2 = Vec3::new(-1.0, 0.0, 0.0).normalize();
        
        // Test case 1: Moving toward each other, will collide
        let (collide1, collide2) = check_if_collide_after_moving(
            &collider1, position1, direction1, 
            &collider2, position2, direction2, 
            10.0
        );
        assert!(collide1);
        assert!(collide2);
        
        // Test case 2: Moving toward each other, but not far enough to collide
        let (collide1, collide2) = check_if_collide_after_moving(
            &collider1, position1, direction1, 
            &collider2, position2, direction2, 
            5.0
        );
        assert!(!collide1);
        assert!(!collide2);
        
        // Test case 3: Moving away from each other
        let direction1_away = Vec3::new(-1.0, 0.0, 0.0).normalize();
        let direction2_away = Vec3::new(1.0, 0.0, 0.0).normalize();
        
        let (collide1, collide2) = check_if_collide_after_moving(
            &collider1, position1, direction1_away, 
            &collider2, position2, direction2_away, 
            10.0
        );
        assert!(!collide1);
        assert!(!collide2);
    }
    
    #[test]
    fn test_get_circle_collider_from_actual_collider() {
        // Test case 1: Cuboid collider
        let cuboid_collider = Collider::cuboid(3.0, 4.0);
        let circle_collider = get_circle_collider_from_actual_collider(&cuboid_collider);
        
        assert!(circle_collider.is_some());
        let circle = circle_collider.unwrap();
        assert!(circle.as_ball().is_some());
        assert_eq!(circle.as_ball().unwrap().radius(), 5.0); // sqrt(3^2 + 4^2) = 5
        
        // Test case 2: Capsule collider
        let capsule_collider = Collider::capsule_y(4.0, 3.0); // half_height = 4.0, radius = 3.0
        let circle_collider = get_circle_collider_from_actual_collider(&capsule_collider);
        
        assert!(circle_collider.is_some());
        let circle = circle_collider.unwrap();
        assert!(circle.as_ball().is_some());
        assert_eq!(circle.as_ball().unwrap().radius(), 7.0); // half_height + radius = 4.0 + 3.0 = 7.0
        
        // Test case 3: Ball collider (should return None as it's already a circle)
        let ball_collider = Collider::ball(5.0);
        let circle_collider = get_circle_collider_from_actual_collider(&ball_collider);
        
        assert!(circle_collider.is_none());
    }

}