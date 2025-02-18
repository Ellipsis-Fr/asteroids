use std::ops::Sub;

use bevy::prelude::*;
use bevy_rapier2d::{na::Rotation, prelude::*};

use super::{components::{AIState, DetectionSensor, Enemy, EnemyState, FakeEntities, Meteor, Player}, screen_overflow, wave::Wave, GameTextures, WinSize, ENEMY_SIZE, SPRITE_SCALE};


pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            enemy_spawn_system.run_if(can_spawn_enemy),
            // enemy_detection_system,
            // enemy_ai_system.after(enemy_detection_system),
            enemy_ai_system,
            enemy_movement_system,
        ));
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
    ));
}

fn enemy_detection_system(
    mut collision_events: EventReader<CollisionEvent>,
    sensor_query: Query<&Parent, With<DetectionSensor>>,
    mut enemy_query: Query<(&Transform, &mut AIState)>,
    meteor_query: Query<(&Transform, &Velocity), With<Meteor>>,
) {
    for (enemy_transform, mut ai_state) in enemy_query.iter_mut() {
        
        // Check for meteors on collision path
        for (meteor_transform, meteor_velocity) in meteor_query.iter() {
            let to_meteor = meteor_transform.translation - enemy_transform.translation;
            let distance = to_meteor.length();
            
            // Simple prediction - check if meteor is moving towards enemy
            if distance < 500.0 {  // Detection radius
                let meteor_direction = meteor_velocity.linvel.extend(0.0).normalize();
                let dot_product = meteor_direction.dot(to_meteor.normalize());
                
                // If meteor is moving towards enemy (dot product > 0)
                if dot_product > 0.0 {
                    ai_state.state = EnemyState::Dodge;
                    break;  // Found a threatening meteor, no need to check others
                }
            }
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
                EnemyState::Chase
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
                    let mut dot_product = enemy_forward.normalize().dot(direction);
                    if dot_product < 0. {
                        dbg!(dot_product);
                    }
                    
                    if dot_product < -1. {
                        dot_product = -1.;
                    } else if dot_product > 1. {
                        dot_product = 1.;
                    }
                    let mut angle_from_dot = dot_product.acos();
                    if dot_product < 0. && angle_from_dot.to_degrees() > 90. {
                        angle_from_dot = angle_from_dot - 180_f32.to_radians();
                    }
                    dbg!(angle_from_dot.to_degrees());
                    // dbg!(dot_product);
                    enemy_transform.rotate(Quat::from_rotation_z(angle_from_dot));
                    if (angle_from_dot.to_degrees() > -40. && angle_from_dot.to_degrees() < 40.) || (angle_from_dot.to_degrees() > 160. && angle_from_dot.to_degrees() < 180.) {
                    }


                    // enemy_transform.rotation = Quat::from_rotation_z(angle_from_dot);

                    // enemy_transform.rotate_z(angle * -1.);
                    // enemy_transform.rotate(Quat::from_rotation_z(angle));
                    // enemy_transform.translation += direction * enemy.speed * time.delta_seconds();
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