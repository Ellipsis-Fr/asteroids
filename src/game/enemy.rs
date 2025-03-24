use std::{collections::{BTreeMap, HashMap}, ops::Sub};

use bevy::prelude::*;
use bevy_rapier2d::{na::Rotation, prelude::*};

use super::{components::{AIState, DetectionSensor, Enemy, EnemyState, FakeEntities, Meteor, Player}, screen_overflow, wave::Wave, GameTextures, WinSize, ENEMY_SIZE, SPRITE_SCALE};

#[derive(Debug)]
struct Threat {
    position: Vec3,
    distance: f32,
    dot_product: f32,
}

#[derive(Default)]
struct DetectedThreats {
    threats_in_current_position: BTreeMap<Entity, Threat>,
    threats_in_futur_position: BTreeMap<Entity, Threat>,
}

#[derive(Resource, Default)]
struct DetectedThreatsByEnemies(pub HashMap<Entity, DetectedThreats>);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DetectedThreatsByEnemies>()
            .add_systems(Update, (
                enemy_spawn_system.run_if(can_spawn_enemy),
                // enemy_detection_system,
                // enemy_ai_system.after(enemy_detection_system),
                enemy_ai_system,
                enemy_movement_system,
            )
        );
    }
}

fn can_spawn_enemy(wave_resource: Res<Wave>) -> bool {
    wave_resource.get_meteors().len() <= 1 && wave_resource.get_enemies() > 0
}

fn enemy_spawn_system(mut commands: Commands, game_textures: Res<GameTextures>, mut wave_resource: ResMut<Wave>) {
    wave_resource.decrease_enemies();

    commands.spawn((
        SpriteBundle {
            texture: game_textures.enemy.clone(),
            transform: Transform {
                translation: Vec3 { x: 0., y: 150., z: 10. },
                rotation: Quat::from_rotation_z(180_f32.to_radians()),
                scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                ..Default::default()
            },
            sprite: Sprite {
                // flip_x: true,
                // flip_y: true,
                ..default()
            },
            ..default()
        },
        Enemy {
            speed: 100.,
            detection_range: 500.,
            attack_range: 150.,
            health: 100.0,
        },
        AIState {
            state: EnemyState::Idle,
        },
        RigidBody::KinematicPositionBased,
        Collider::cuboid(ENEMY_SIZE.0 / 2., ENEMY_SIZE.1 / 2.),
        FakeEntities(vec![])
    )).with_children(|parent| {
        // Spawn sensor as child
        parent.spawn((
            Collider::ball(500.), // Adjust radius as needed
            Sensor,
            DetectionSensor, // todo: No used, to delete
            Transform::default(),
            GlobalTransform::default(),
        ));

        parent.spawn((
            Collider::ball(300.), // Adjust radius as needed
            Sensor,
            DetectionSensor,
            Transform::default(),
            GlobalTransform::default(),
        ));
    });
}

fn enemy_detection_system(
    win_size: Res<WinSize>,
    time: Res<Time>,
    mut detected_threats_by_enemies: ResMut<DetectedThreatsByEnemies>,
    mut query_enemies: Query<(Entity, &Enemy, &Transform, &mut AIState, &Collider)>,
    query_projectiles: Query<(Entity, &Transform, &Velocity, &Collider)>,
) {
    detected_threats_by_enemies.0.clear();

    for (enemy_entity, enemy_struct, enemy_transform, mut ai_state, enemy_collider) in query_enemies.iter_mut() {
        
        let mut detected_threats = DetectedThreats::default();

        let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
        let enemy_futur_position = enemy_transform.translation + enemy_direction * enemy_struct.speed * time.delta_seconds();

        for (projectile_entity, projectile_transform, projectile_velocity, projectile_collider) in query_projectiles.iter() {
            if (enemy_entity == projectile_entity) { continue }
            
            let projectile_nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, projectile_transform.translation);
            let to_projectile = projectile_nearest_position - enemy_transform.translation;
            let actual_distance = to_projectile.length();

            if actual_distance > 500. { continue }
            else {
                let projectile_direction = projectile_velocity.linvel.extend(0.0).normalize();

                // ajouter ici pour calcul ensuite :
                // - calcul position projectile en direction du vaisseau avec time
                let projectile_futur_position = projectile_transform.translation + projectile_velocity.linvel.extend(0.) * time.delta_seconds();
                let projectile_futur_position_nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, projectile_futur_position);
                let to_projectile_futur_position_without_enemy_moving = projectile_futur_position_nearest_position - enemy_transform.translation;
                let futur_distance_without_enemy_moving = to_projectile_futur_position_without_enemy_moving.length();
                let to_projectile_futur_position_with_enemy_moving = projectile_futur_position_nearest_position - enemy_futur_position;
                let futur_distance_with_enemy_moving = to_projectile_futur_position_with_enemy_moving.length();

                let dot_product_actual_position = projectile_direction.dot(to_projectile.normalize()).clamp(-1., 1.);
                let dot_product_futur_position_without_enemy_moving = projectile_direction.dot(to_projectile_futur_position_without_enemy_moving.normalize()).clamp(-1., 1.);
                let dot_product_futur_position_with_enemy_moving = projectile_direction.dot(to_projectile_futur_position_with_enemy_moving.normalize()).clamp(-1., 1.);

                if actual_distance > 300. {
                    //todo: calcul pour vérifier si la rotation du vaisseau ne risque pas d'entrer en colision, si pas de risque alors on peut continuer à vérifier condition pour ne pas tenir compte de ce projectile
                    if true {
                        if dot_product_actual_position <= 0. {
                            if actual_distance < futur_distance_without_enemy_moving && actual_distance < futur_distance_with_enemy_moving { continue }
                        } else {
                            if dot_product_actual_position > dot_product_futur_position_without_enemy_moving && dot_product_actual_position > dot_product_futur_position_with_enemy_moving { continue }
                        }
                    }

                }

                detected_threats.threats_in_current_position.insert(
                    projectile_entity,
                    Threat {
                        position: projectile_nearest_position,
                        distance: actual_distance,
                        dot_product: dot_product_actual_position 
                    }
                );

                detected_threats.threats_in_futur_position.insert(
                    projectile_entity,
                    Threat {
                        position: projectile_futur_position_nearest_position,
                        distance: futur_distance_without_enemy_moving,
                        dot_product: dot_product_futur_position_without_enemy_moving 
                    }
                );
                
            
                // calcul secondaire :
                // - déterminer avec leur collider, et rotation, respectifs s'il peut y avoir un choc (se baser sur solutions rapier si possible)
                // - si oui alors même si le dot_product est < 0. on conservera ce risque
                // - si non alors c'est le dot_product qui jugera s'il faut ou non conserver ces informations
            }
        }

        if !detected_threats.threats_in_current_position.is_empty() {
            ai_state.state = EnemyState::Dodge;
            detected_threats_by_enemies.0.insert(enemy_entity.clone(), detected_threats);
        }
    }
}

fn enemy_ai_system(
    win_size: Res<WinSize>,
    mut enemy_query: Query<(&Transform, &Enemy, &mut AIState)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>
) {
    if let Ok(player_transform) = player_query.get_single() {
        for (enemy_transform, enemy, mut ai_state) in enemy_query.iter_mut() {
            let nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, player_transform.translation);
            let distance = enemy_transform.translation.distance(nearest_position);
            // there caution to take in account the screen limit
            // I will need it to Chase and Attack action (allow enemy fire to pass trougth the screen)

            if ai_state.state == EnemyState::Dodge {
                continue;
            }
            // Update AI state based on distance to player
            ai_state.state = if distance <= enemy.attack_range {
                EnemyState::Attack
            } else if distance <= enemy.detection_range {
                EnemyState::Idle
            } else {
                EnemyState::Patrol
            };
        }
    }
}

// Movement system
fn enemy_movement_system(
    win_size: Res<WinSize>,
    time: Res<Time>,
    mut enemy_query: Query<(&mut Transform, &Enemy, &AIState)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        for (mut enemy_transform, enemy, ai_state) in enemy_query.iter_mut() {
            match ai_state.state {
                EnemyState::Chase => {
                    // Move towards player
                    let nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, player_transform.translation);
                    let direction = (nearest_position - enemy_transform.translation).normalize();

                    let enemy_forward = enemy_transform.rotation * Vec3::Y; // This gets the forward vector based on rotation
                    let cross_product = enemy_forward.cross(direction);
                    
                    let dot_product = enemy_forward.normalize().dot(direction).clamp(-1., 1.);
                    let mut angle = dot_product.acos().clamp(0., 10_f32.to_radians());
                    
                    let signed_angle = if cross_product.z < 0.0 { -angle } else { angle };
                    
                    enemy_transform.rotate(Quat::from_rotation_z(signed_angle));
                    // todo: edit translation according to the angle
                    enemy_transform.translation += direction * enemy.speed * time.delta_seconds();
                }
                EnemyState::Dodge => {

                }
                EnemyState::Patrol => {
                    // Implement patrol behavior (e.g., moving in a pattern)
                    // This is a simple back-and-forth movement
                    let patrol_movement = (time.elapsed_seconds() * 2.0).sin() * enemy.speed * 0.5 * time.delta_seconds();
                    enemy_transform.translation.x += patrol_movement;
                }
                EnemyState::Attack => {
                    // Implement attack behavior
                    // For example, spawn projectiles or deal damage to player
                }
                EnemyState::Idle => {
                    // Do nothing while idle
                }
            }
        }
    }
}