use std::{collections::{BTreeMap, HashMap}, ops::Sub, time::Instant};

use bevy::{prelude::*, utils::hashbrown::HashSet};
use bevy_rapier2d::{na::Rotation, prelude::*};

use crate::game::collision::check_if_collide;

use super::{collision, components::{AIState, DetectionSensor, Enemy, EnemyState, FakeEntities, LaserTimer, Meteor, Player}, screen_overflow, wave::Wave, GameTextures, WinSize, ENEMY_SIZE, SPRITE_SCALE};

const DODGE_ACCURACY: u32 = 4; // enemy movement decomposition number

#[derive(Debug, Eq, PartialEq)]
enum DodgeStatus {
    Free,
    MustMove,
    CriticalShoot(RotationDirection, u32)
}

#[derive(Debug, Eq, PartialEq, Hash)]
enum MovementOption {
    Stationary,
    Move(u32),                                      // has a value depending on the DODGE_ACCURACY
    Rotation(RotationDirection, u32),               // indicates direction of rotation and its angle
    MoveAndRotation(u32, RotationDirection, u32)
}

impl MovementOption {
    fn all_variants() -> Vec<MovementOption> {
        vec![
            MovementOption::Stationary,
            MovementOption::Move(0),
            MovementOption::Rotation(RotationDirection::Clockwise, 0),
            MovementOption::Rotation(RotationDirection::CounterClockwise, 0),
            MovementOption::MoveAndRotation(0, RotationDirection::Clockwise, 0),
            MovementOption::MoveAndRotation(0, RotationDirection::CounterClockwise, 0)
        ]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum RotationDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug)]
struct Threat {
    position: Vec3,
    rotation: Quat,                 // useful exlusively for ship threats (cause laser are too small and meteor are circular) 
    distance: f32,
    velocity: Vec2,
    threat_dot_product: f32,        // dot product from threat POV
    threatened_dot_product: f32,    // dot product from enemy POV
    collider: Collider
}

#[derive(Resource, Default)]
struct DetectedThreatsByEnemies(pub HashMap<Entity, Vec<Threat>>);

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DetectedThreatsByEnemies>()
            .add_systems(Update, (
                enemy_spawn_system.run_if(can_spawn_enemy),
                enemy_ai_system,
                enemy_movement_system,
            ).chain()
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
            detection_range: 600.,
            attack_range: 250.,
            health: 100.,
            shoot_delay_seconds: 3.
        },
        AIState {
            state: EnemyState::Idle,
        },
        LaserTimer::default(),
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

        parent.spawn((
            Collider::ball((Vec2::new(68., 42.)).length()), // Adjust radius as needed
            Sensor,
            DetectionSensor,
            Transform::default(),
            GlobalTransform::default(),
        ));
    });
}

fn enemy_ai_system(
    win_size: Res<WinSize>,
    time: Res<Time>,
    mut detected_threats_by_enemies: ResMut<DetectedThreatsByEnemies>,
    mut enemy_query: Query<(Entity, &Enemy, &Transform, &Collider, &mut AIState)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    query_projectiles: Query<(Entity, &Transform, &Velocity, &Collider), Without<Player>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        detected_threats_by_enemies.0.clear();

        for (enemy_entity, enemy, enemy_transform, enemy_collider, mut ai_state) in enemy_query.iter_mut() {
            if let Some(detected_threats) = detect_threats(&win_size, &time, (enemy_entity, enemy, enemy_transform, enemy_collider), query_projectiles.iter().collect()) {
                ai_state.state = EnemyState::Dodge;
                detected_threats_by_enemies.0.insert(enemy_entity.clone(), detected_threats);
            } else {
                let nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, player_transform.translation);
                let distance = enemy_transform.translation.distance(nearest_position);
                // there caution to take in account the screen limit
                // I will need it to Chase and Attack action (allow enemy fire to pass trougth the screen)
    
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
}

fn detect_threats(
    win_size: &Res<WinSize>,
    time: &Res<Time>,
    enemy_information: (Entity, &Enemy, &Transform, &Collider),
    query_projectiles: Vec<(Entity, &Transform, &Velocity, &Collider)>,
) -> Option<Vec<Threat>> {
    let mut detected_threats = Vec::new();
    
    let (enemy_entity, enemy_struct, enemy_transform, enemy_collider) = enemy_information;
    let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
    let enemy_futur_position = enemy_transform.translation + enemy_direction * enemy_struct.speed * time.delta_seconds();

    for (projectile_entity, projectile_transform, projectile_velocity, projectile_collider) in query_projectiles {
        if (enemy_entity == projectile_entity) { continue }
        
        let projectile_nearest_position = screen_overflow::get_nearest_position(&win_size, enemy_transform.translation, projectile_transform.translation);
        let to_projectile = projectile_nearest_position - enemy_transform.translation;
        let actual_distance = to_projectile.length();

        if actual_distance > 250. { continue }
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

            let projectile_dot_product_actual_position = projectile_direction.dot(to_projectile.normalize()).clamp(-1., 1.);
            let projectile_dot_product_futur_position_without_enemy_moving = projectile_direction.dot(to_projectile_futur_position_without_enemy_moving.normalize()).clamp(-1., 1.);
            let projectile_dot_product_futur_position_with_enemy_moving = projectile_direction.dot(to_projectile_futur_position_with_enemy_moving.normalize()).clamp(-1., 1.);

            if actual_distance > 150. {
                //todo: calcul pour vérifier si la rotation du vaisseau ne risque pas d'entrer en colision, si pas de risque alors on peut continuer à vérifier condition pour ne pas tenir compte de ce projectile
                if true {
                    if projectile_dot_product_actual_position <= 0. {
                        if actual_distance < futur_distance_without_enemy_moving && actual_distance < futur_distance_with_enemy_moving { continue }
                    } else {
                        if projectile_dot_product_actual_position > projectile_dot_product_futur_position_without_enemy_moving && projectile_dot_product_actual_position > projectile_dot_product_futur_position_with_enemy_moving { continue }
                    }
                }
            }

            detected_threats.push(
                Threat {
                    position: projectile_nearest_position,
                    rotation: enemy_transform.rotation,
                    distance: actual_distance,
                    velocity: projectile_velocity.linvel,
                    threat_dot_product: projectile_dot_product_actual_position,
                    threatened_dot_product: enemy_direction.dot(to_projectile.normalize()).clamp(-1., 1.),
                    collider: projectile_collider.clone()
                }
            );            
        
            // calcul secondaire :
            // - déterminer avec leur collider, et rotation, respectifs s'il peut y avoir un choc (se baser sur solutions rapier si possible)
            // - si oui alors même si le dot_product est < 0. on conservera ce risque
            // - si non alors c'est le dot_product qui jugera s'il faut ou non conserver ces informations
        }
    }

    if !detected_threats.is_empty() {
        Some(detected_threats)   
    } else {
        None
    }
}

// Movement system
fn enemy_movement_system(
    win_size: Res<WinSize>,
    time: Res<Time>,
    detected_threats_by_enemies: Res<DetectedThreatsByEnemies>,
    mut enemy_query: Query<(Entity, &mut Transform, &Collider, &Enemy, &AIState, &mut LaserTimer)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>
) {
    if let Ok(player_transform) = player_query.get_single() {
        for (enemy_entity, mut enemy_transform, enemy_collider, enemy, ai_state, mut laser_timer) in enemy_query.iter_mut() {
            match ai_state.state {
                EnemyState::Chase => chase(&win_size, &time, &mut enemy_transform, enemy, player_transform),
                EnemyState::Dodge => dodge(&time, &mut enemy_transform, enemy_collider, enemy, &mut laser_timer, detected_threats_by_enemies.0.get(&enemy_entity).unwrap()),
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

fn chase(win_size: &Res<WinSize>, time: &Res<Time>, enemy_transform: &mut Transform, enemy: &Enemy, player_transform: &Transform) {
    // Move towards player
    let nearest_position = screen_overflow::get_nearest_position(win_size, enemy_transform.translation, player_transform.translation);
    let direction = (nearest_position - enemy_transform.translation).normalize();

    // This gets the forward vector based on rotation
    let enemy_forward = enemy_transform.rotation * Vec3::Y;
    let cross_product = enemy_forward.cross(direction);
                    
    let dot_product = enemy_forward.normalize().dot(direction).clamp(-1., 1.);
    let mut angle = dot_product.acos().clamp(0., 10_f32.to_radians());
                    
    let signed_angle = if cross_product.z < 0.0 { -angle } else { angle };
                    
    enemy_transform.rotate(Quat::from_rotation_z(signed_angle));
    // todo: edit translation according to the angle
    enemy_transform.translation += direction * enemy.speed * time.delta_seconds();
}

fn dodge(time: &Res<Time>, enemy_transform: &mut Mut<Transform>, enemy_collider: &Collider, enemy: &Enemy, enemy_laser_timer: &mut LaserTimer, detected_threats: &Vec<Threat>) {
    let mut dodge_status = DodgeStatus::Free;
    let mut forbidden_movements = HashSet::new();
    let mut possible_movements_and_notation_for_all_threats = Vec::new(); // liste de listes de mouvements possible associer à des notes : Vec<Vec<(MovementOption, i8)>>

    let can_shoot = enemy_laser_timer.0.finished();
    let enemy_collider_circle = collision::get_circle_collider_from_actual_collider(enemy_collider).unwrap();

    for threat in detected_threats {
        let to_threat = threat.position - enemy_transform.translation;
        let threat_futur_position_in_one_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds();
        let threat_futur_position_in_two_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds() * 2;
        if collision::check_if_collide(&enemy_collider_circle, enemy_transform.translation.truncate(), &threat.collider, threat_futur_position_in_one_frame.truncate()) {
            // verifier direction du vaisseau :
            //  - si elle va vers le projectile alors passer en DodgeStatus::CriticalShoot
            //  - si elle diffère d'au moins 90° alors :
            //      - passer en DodgeStatus::MustMove,
            //      - indiquer les mouvements interdits suivants :
            //          - MovementOption::Stationary
            //          - MovementOption::Rotation dont l'angle fait que l'angle entre la direction du vaisseau et le projectile diminue
            //          - MovementOption::MoveAndRotation avec toute les valeurs de deplacement possible selon const DODGE_ACCURACY associé aux rotations posant problèmes
            
            if threat.threatened_dot_product > 0. {
                // si angle entre direction du vaisseau et position du météor inf. ou égale à 40° alors tourner et tirer
                // sinon ne rien faire

                // ! Vérifier calcul (pour obtenir l'angle, et impact du signe)
                let angle = get_angle_in_radian_between_two_entities(enemy_transform.rotation, to_threat);
                dodge_status = if angle < 0. {
                    DodgeStatus::CriticalShoot(RotationDirection::Clockwise, angle.abs() as u32)
                } else {
                    DodgeStatus::CriticalShoot(RotationDirection::CounterClockwise, angle as u32)
                };
                break;
            } else {
                dodge_status = DodgeStatus::MustMove;
                
                forbidden_movements.insert(MovementOption::Stationary);
                
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::Clockwise, 10));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::Clockwise, 20));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::Clockwise, 30));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::Clockwise, 40));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::CounterClockwise, 10));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::CounterClockwise, 20));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::CounterClockwise, 30));
                forbidden_movements.insert(MovementOption::Rotation(RotationDirection::CounterClockwise, 40));
                
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 1 / DODGE_ACCURACY, RotationDirection::Clockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 2 / DODGE_ACCURACY, RotationDirection::Clockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 3 / DODGE_ACCURACY, RotationDirection::Clockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 4 / DODGE_ACCURACY, RotationDirection::Clockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 1 / DODGE_ACCURACY, RotationDirection::CounterClockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 2 / DODGE_ACCURACY, RotationDirection::CounterClockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 3 / DODGE_ACCURACY, RotationDirection::CounterClockwise, 10));
                forbidden_movements.insert(MovementOption::MoveAndRotation(enemy.speed as u32 * 4 / DODGE_ACCURACY, RotationDirection::CounterClockwise, 10));


                // let mut enemy_transform_cloned = enemy_transform.clone();
                // enemy_transform_cloned.rotate(Quat::from_rotation_z(10_f32.to_radians()));
                // let enemy_direction = (enemy_transform_cloned.rotation * Vec3::Y).normalize();
                // let enemy_dot_product = enemy_direction.dot(to_projectile.normalize()).clamp(-1., 1.);
            }
        } else if collision::check_if_collide(&enemy_collider_circle, enemy_transform.translation.truncate(), &threat.collider, threat_futur_position_in_two_frame.truncate()) {
            dodge_status = DodgeStatus::MustMove;

            forbidden_movements.insert(MovementOption::Stationary);
        }

        let mut possible_movements_and_notation = Vec::new(); // liste de listes de mouvements possible associer à des notes : Vec<Vec<(MovementOption, i8)>>

        // Iterer sur les deplacement possibles en sautant ceux placés dans la liste des dplcts impossibles
        for movement_option in MovementOption::all_variants() {
            match movement_option {
                MovementOption::Stationary => {
                    if !forbidden_movements.contains(&MovementOption::Stationary) && dodge_status == DodgeStatus::Free {
                        possible_movements_and_notation.push((movement_option, 0));
                    }
                },
                MovementOption::Move(_) => {
                    let enemy_forward = enemy_transform.rotation * Vec3::Y;
                    // fonction de DODGE_ACCURACY tester plusieurs deplacements
                    for move_factor in 1..=DODGE_ACCURACY {
                        let enemy_move = enemy.speed as u32 * move_factor / DODGE_ACCURACY;

                        if !forbidden_movements.contains(&MovementOption::Move(enemy_move)) {
                            // todo: voir pour conserver ces données, pour ne pas avoir à les recalculés pour études avec les menaces d'après
                            let enemy_futur_position = enemy_transform.translation + enemy_forward * enemy_move as f32 * time.delta_seconds();
                            if check_if_collide(&enemy_collider_circle, enemy_futur_position.truncate(), &threat.collider, threat.position.truncate()) {
                                forbidden_movements.insert(MovementOption::Move(enemy_move));
                                break;
                            } else {
                                let mut note = move_factor;
                                if threat.distance < enemy_futur_position.distance(threat.position) {
                                    note += 1;
                                } // todo: peut être voir aussi pour ajouter des points si le dot évolue dans le bon sens
                                possible_movements_and_notation.push((MovementOption::Move(enemy_move), note));
                            }
                        }
                    }
                },
                MovementOption::Rotation(rotation_direction, _) => todo!(),
                MovementOption::MoveAndRotation(_, rotation_direction, _) => todo!(),
            }
        }

        possible_movements_and_notation_for_all_threats.push(possible_movements_and_notation);
    }

    match dodge_status {
        DodgeStatus::CriticalShoot(rotation_direction, angle) => todo!(), // action de tire sur cible
        _ => todo!() // action de notation puis de réalisation de l'action
    }
}

fn get_angle_in_radian_between_two_entities(rotation: Quat, direction: Vec3) -> f32 {
    // This gets the forward vector based on rotation
    let entity_forward = rotation * Vec3::Y;
    let cross_product = entity_forward.cross(direction);
                    
    let dot_product = entity_forward.normalize().dot(direction).clamp(-1., 1.);
    let mut angle = dot_product.acos();

    if cross_product.z < 0.0 { -angle } else { angle }
}