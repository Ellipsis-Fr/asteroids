use std::{collections::{BTreeMap, HashMap}, ops::Sub, time::{Duration, Instant}};

use bevy::{a11y::accesskit::Action, prelude::*, utils::hashbrown::HashSet};
use bevy_rapier2d::{na::Rotation, prelude::*};

use crate::game::collision::check_if_collide;

use super::{collision::{self, check_if_collide_after_moving}, components::{AIState, DetectionSensor, Enemy, EnemyState, FakeEntities, LaserTimer, Meteor, Player}, screen_overflow, wave::Wave, GameTextures, WinSize, ENEMY_SIZE, SPRITE_SCALE};

const DODGE_ACCURACY: u32 = 1; // enemy movement decomposition number
const ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING: [u32; 4] = [10, 20, 30, 40];
const ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITH_MOVING: u32 = 10; 

#[derive(Debug, Eq, PartialEq)]
enum DodgeStatus {
    Free,
    MustMove,
    CriticalShoot(RotationDirection, u32),
    Impossible
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
enum ActionOption {
    Movement(MovementOption),
    Shoot((RotationDirection, u32), u32),
}

impl ActionOption {
    fn all_variants() -> Vec<ActionOption> {
        let movement_options = MovementOption::all_variants()
            .into_iter()
            .map(ActionOption::Movement)
            .collect::<Vec<_>>();

        let shoot_options = vec![ActionOption::Shoot((RotationDirection::Clockwise, 0), 0)];
        
        vec![movement_options, shoot_options].concat()
    }
}


#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
enum MovementOption {
    Stationary,
    Move(u32),                                      // has a value depending on the DODGE_ACCURACY
    Rotation(RotationDirection, u32),               // indicates direction of rotation and its angle
    MoveAndRotation(u32, RotationDirection, u32)
}

impl MovementOption {
    fn all_variants() -> Vec<MovementOption> {
        let all_move = (1..=DODGE_ACCURACY).into_iter().map(|move_factor| MovementOption::Move(move_factor)).collect::<Vec<_>>();
        let all_rotations = ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING
            .into_iter()
            .flat_map(|angle| [
                MovementOption::Rotation(RotationDirection::Clockwise, angle),
                MovementOption::Rotation(RotationDirection::CounterClockwise, angle)
            ])
            .collect::<Vec<_>>();

        let all_move_and_rotations = (1..=DODGE_ACCURACY)
            .into_iter()
            .flat_map(|move_factor| [
                MovementOption::MoveAndRotation(move_factor, RotationDirection::Clockwise, ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITH_MOVING),
                MovementOption::MoveAndRotation(move_factor, RotationDirection::CounterClockwise, ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITH_MOVING)
            ])
            .collect::<Vec<_>>();

        // Chain all iterators together and collect into a single Vec
        std::iter::once(MovementOption::Stationary)
            .chain(all_move)
            .chain(all_rotations)
            .chain(all_move_and_rotations)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum RotationDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Component)]
struct LastDodgeRotation(Option<RotationDirection>);

#[derive(Debug)]
struct Threat {
    position: Vec3,
    direction: Vec3,
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
        LastDodgeRotation(None),
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
    mut enemy_query: Query<(Entity, &Enemy, &Transform, &Collider, &mut AIState, &mut LastDodgeRotation)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
    query_projectiles: Query<(Entity, &Transform, &Velocity, &Collider), Without<Player>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        detected_threats_by_enemies.0.clear();

        for (enemy_entity, enemy, enemy_transform, enemy_collider, mut ai_state, mut last_dodge_rotation) in enemy_query.iter_mut() {
            if let Some(detected_threats) = detect_threats(&win_size, &time, (enemy_entity, enemy, enemy_transform, enemy_collider), query_projectiles.iter().collect()) {
                ai_state.state = EnemyState::Dodge;
                detected_threats_by_enemies.0.insert(enemy_entity.clone(), detected_threats);
            } else {
                last_dodge_rotation.0 = None;
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

            match check_if_collide_after_moving(enemy_collider, enemy_transform.translation, enemy_direction, projectile_collider, projectile_transform.translation, projectile_direction, actual_distance) {
                (false, false) => continue,
                (true, false) => {
                    // println!("collision si vaisseau bouge");
                    if projectile_velocity.linvel.length() > enemy_struct.speed {
                        // println!("mais pas rattrapable");
                        continue;
                    }
                },
                _ => (),
                (false, true) => println!("collision si météore bouge"),
                (true, true) => println!("collision peu importe qui bouge"),
                // _ => {
                //     println!()
                //     // dbg!(enemy_transform.translation.truncate(), enemy_direction, projectile_transform.translation.truncate(), projectile_direction, actual_distance);
                // } 
            }

            // todo: ajouter condition de vitesse, vérifier si le vaisseau est réellement en capacité de rattraper le météor, sinon le virer des risques

            if actual_distance > 150. {
                //todo: calcul pour vérifier si la rotation du vaisseau ne risque pas d'entrer en colision, si pas de risque alors on peut continuer à vérifier condition pour ne pas tenir compte de ce projectile
                if true {
                    if projectile_dot_product_actual_position <= 0. {
                        if actual_distance < futur_distance_without_enemy_moving && actual_distance < futur_distance_with_enemy_moving { continue }
                    } 
                    // else {
                    //     if projectile_dot_product_actual_position > projectile_dot_product_futur_position_without_enemy_moving && projectile_dot_product_actual_position > projectile_dot_product_futur_position_with_enemy_moving { continue }
                    // }
                }
            }

            detected_threats.push(
                Threat {
                    position: projectile_nearest_position,
                    direction: projectile_direction,
                    rotation: enemy_transform.rotation,
                    distance: actual_distance,
                    velocity: projectile_velocity.linvel,
                    threat_dot_product: projectile_dot_product_actual_position,
                    threatened_dot_product: enemy_direction.dot(to_projectile.normalize()).clamp(-1., 1.),
                    collider: projectile_collider.clone()
                }
            );
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
    mut enemy_query: Query<(Entity, &mut Transform, &Collider, &Enemy, &AIState, &mut LastDodgeRotation, &mut LaserTimer)>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>
) {
    let now = Instant::now();
    
    if let Ok(player_transform) = player_query.get_single() {
        for (enemy_entity, mut enemy_transform, enemy_collider, enemy, ai_state, mut last_dodge_rotation, mut laser_timer) in enemy_query.iter_mut() {
            match ai_state.state {
                EnemyState::Chase => chase(&win_size, &time, &mut enemy_transform, enemy, player_transform),
                EnemyState::Dodge => dodge(now, &time, &mut enemy_transform, enemy_collider, enemy, &mut last_dodge_rotation, &mut laser_timer, detected_threats_by_enemies.0.get(&enemy_entity).unwrap()),
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
    let mut enemy_direction = enemy_transform.rotation * Vec3::Y;
    let cross_product = enemy_direction.cross(direction);
                    
    let dot_product = enemy_direction.normalize().dot(direction).clamp(-1., 1.);
    let mut angle = dot_product.acos().clamp(0., 20_f32.to_radians());
                    
    let signed_angle = if cross_product.z < 0.0 { -angle } else { angle };
                    
    enemy_transform.rotate(Quat::from_rotation_z(signed_angle));
    enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();

    enemy_transform.translation += enemy_direction * enemy.speed * time.delta_seconds();
}

fn dodge(now: Instant, time: &Res<Time>, enemy_transform: &mut Mut<Transform>, enemy_collider: &Collider, enemy: &Enemy, last_dodge_rotation: &mut LastDodgeRotation, enemy_laser_timer: &mut LaserTimer, detected_threats: &Vec<Threat>) {
    let mut dodge_status = DodgeStatus::Free;
    let mut forbidden_movements = init_forbidden_movements(last_dodge_rotation);
    let mut possible_actions_and_notation_for_all_threats = Vec::new(); // liste de listes de mouvements possible associer à des notes : Vec<Vec<(MovementOption, i8)>>
    let mut enemy_futures_positions_by_movement_option = HashMap::new();

    let can_shoot = enemy_laser_timer.0.tick(time.delta()).finished();
    let enemy_actual_direction = (enemy_transform.rotation * Vec3::Y).normalize();
    let enemy_collider_circle = collision::get_circle_collider_from_actual_collider(enemy_collider).unwrap();

    for threat in detected_threats {
        let to_threat = threat.position - enemy_transform.translation;
        let threat_futur_position_in_one_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds();
        let threat_futur_position_in_two_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds() * 2.;
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
                dodge_status = if angle.abs() < (ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING[ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING.len() - 1] as f32).to_radians() {
                    if angle > 0. {
                        DodgeStatus::CriticalShoot(RotationDirection::Clockwise, angle.to_degrees() as u32)
                    } else {
                        DodgeStatus::CriticalShoot(RotationDirection::CounterClockwise, angle.abs().to_degrees() as u32)
                    }
                } else {
                    DodgeStatus::Impossible
                };
                break;
            } else {
                dodge_status = DodgeStatus::MustMove;

                for movement_option in MovementOption::all_variants() {
                    match movement_option {
                        MovementOption::Stationary => { forbidden_movements.insert(MovementOption::Stationary); },
                        MovementOption::Rotation(rotation_direction, angle_degree) => { forbidden_movements.insert(MovementOption::Rotation(rotation_direction, angle_degree)); }
                        MovementOption::MoveAndRotation(move_factor, rotation_direction, angle_degree) => { forbidden_movements.insert(MovementOption::MoveAndRotation(move_factor, rotation_direction, angle_degree)); }
                        _ => ()
                    }
                }
            }
        } else if collision::check_if_collide(&enemy_collider_circle, enemy_transform.translation.truncate(), &threat.collider, threat_futur_position_in_two_frame.truncate()) {
            dodge_status = DodgeStatus::MustMove;

            forbidden_movements.insert(MovementOption::Stationary);
        }

        let mut possible_actions_and_notation = Vec::new(); // liste de listes de mouvements possible associer à des notes : Vec<Vec<(MovementOption, i8)>>
        
        // Iterer sur les deplacement possibles en sautant ceux placés dans la liste des dplcts impossibles
        for action_option in ActionOption::all_variants() {
            match action_option {
                ActionOption::Movement(movement_option) => {
                    match movement_option {
                        MovementOption::Stationary => {
                            if !forbidden_movements.contains(&movement_option) && dodge_status == DodgeStatus::Free {
                                let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
                                match check_if_collide_after_moving(enemy_collider, enemy_transform.translation, enemy_direction, &threat.collider, threat.position, threat.direction, threat.distance) {
                                    (false, false) => possible_actions_and_notation.push((action_option, 3)),
                                    (true, false) => possible_actions_and_notation.push((action_option, 2)),
                                    (false, true) => possible_actions_and_notation.push((action_option, -1)),
                                    _ => (),
                                }






                                // if threat.threat_dot_product < 0. {
                                //     possible_actions_and_notation.push((action_option, 2));
                                // } else {
                                //     let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
                                //     let enemy_future_position = enemy_transform.translation + enemy_direction * threat.distance;


                                //     possible_actions_and_notation.push((action_option, 0));
                                // }
                                // Ajouter des conditions pour ajouter des points cela permettra d'éviter que le vaisseau tourne sur lui
                            }
                        },
                        MovementOption::Move(move_factor) => {
                            if !forbidden_movements.contains(&movement_option) {
                                let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
        
                                let enemy_next_position = match enemy_futures_positions_by_movement_option.get(&MovementOption::Move(move_factor)) {
                                    Some(enemy_futur_position_known) => *enemy_futur_position_known,
                                    None => {
                                        let result = enemy_transform.translation + enemy_direction * ((enemy.speed as u32 * move_factor / DODGE_ACCURACY) as f32) * 1.5 * time.delta_seconds();
                                        // dbg!(enemy_transform.translation, result, enemy_transform.translation - result);
                                        enemy_futures_positions_by_movement_option.insert(MovementOption::Move(move_factor), result);
                                        result
                                    }
                                };
        
                                if check_if_collide(&enemy_collider_circle, enemy_next_position.truncate(), &threat.collider, threat.position.truncate()) {
                                    forbidden_movements.insert(MovementOption::Move(move_factor));
                                    break;
                                } else {
                                    let mut note = move_factor as i32;

                                    match check_if_collide_after_moving(enemy_collider, enemy_transform.translation, enemy_direction, &threat.collider, threat.position, threat.direction, threat.distance) {
                                        (false, true) => {
                                            let to_threat_after_moving_on_threat_distance = (threat.position + threat.direction * threat.distance) - (enemy_transform.translation + enemy_direction * threat.distance);
                                            let dot_product_after_moving_on_threat_distance = threat.direction.dot(to_threat_after_moving_on_threat_distance.normalize()).clamp(-1., 1.);
                                            
                                            if threat.distance < threat.velocity.length() * 2. {
                                                if dot_product_after_moving_on_threat_distance.acos().to_degrees() < 20. {
                                                    note -= 3;
                                                } else if dot_product_after_moving_on_threat_distance > 0. {
                                                    note += 0;
                                                } else {
                                                    note += 2;
                                                }
                                            } else {
                                                note += 1;
                                            }
                                        },
                                        (false, false) => note += 1,
                                        (true, false) => note -= 1,
                                        (true, true) => note -= 2
                                    }
                                    
                                    // let actual_distance = threat.distance;
                                    // let future_distance = enemy_futur_position.distance(threat.position);
                                    if threat.distance < enemy_next_position.distance(threat.position) { // todo! magic number to edit
                                        // dbg!((actual_distance, future_distance)); 
                                        note += 1;
                                    } 
        
                                    // if threat.threat_dot_product > threat.direction.dot((threat.position - enemy_futur_position).normalize()).clamp(-1., 1.) {
                                    //     note += 1;
                                    // }
        
                                    possible_actions_and_notation.push((action_option, note));
                                }
                            }
                        },
                        MovementOption::Rotation(rotation_direction, angle_degree) => {
                            if !forbidden_movements.contains(&movement_option) {
                                let angle_degree = match rotation_direction {
                                    RotationDirection::Clockwise => angle_degree as f32,
                                    RotationDirection::CounterClockwise => angle_degree as f32 * -1.
                                };
                                
                                let enemy_new_direction = {
                                    let mut enemy_transform_cloned = enemy_transform.clone();
                                    enemy_transform_cloned.rotate(Quat::from_rotation_z(angle_degree.to_radians()));
                                    (enemy_transform_cloned.rotation * Vec3::Y).normalize()
                                };

                                let enemy_futur_position = match enemy_futures_positions_by_movement_option.get(&movement_option) {
                                    Some(enemy_futur_position_known) => *enemy_futur_position_known,
                                    None => {
                                        let result = enemy_transform.translation + enemy_new_direction * threat.distance;
                                        enemy_futures_positions_by_movement_option.insert(movement_option, result);
                                        result
                                    }
                                };

                                if check_if_collide(&enemy_collider_circle, enemy_futur_position.truncate(), &threat.collider, threat.position.truncate()) {
                                    let threat_new_position = threat.position + threat.direction * threat.distance;
                                    if check_if_collide(&enemy_collider_circle, enemy_transform.translation.truncate(), &threat.collider, threat_new_position.truncate()) {
                                        let threat_futur_position_in_three_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds() * 3.;
                                        let threat_distance_traveled_by_frame = threat.position.distance(threat_futur_position_in_one_frame);
                                        let threat_distance_remaining_after_three_frame = enemy_transform.translation.distance(threat_futur_position_in_three_frame);
                                        if threat_distance_remaining_after_three_frame < threat_distance_traveled_by_frame {
                                            forbidden_movements.insert(movement_option);
                                        } else {
                                            possible_actions_and_notation.push((action_option, -1));
                                        }
                                    } else {
                                        possible_actions_and_notation.push((action_option, 0));
                                    }
                                } else {
                                    possible_actions_and_notation.push((action_option, angle_degree as i32 / 20));
                                }
                            }
                        },
                        MovementOption::MoveAndRotation(move_factor, rotation_direction, angle_degree) => {
                            if !forbidden_movements.contains(&movement_option) {
                                let angle_degree = match rotation_direction {
                                    RotationDirection::Clockwise => angle_degree as f32,
                                    RotationDirection::CounterClockwise => angle_degree as f32 * -1.
                                };    

                                let enemy_new_direction = {
                                    let mut enemy_transform_cloned = enemy_transform.clone();
                                    enemy_transform_cloned.rotate(Quat::from_rotation_z(angle_degree.to_radians()));
                                    (enemy_transform_cloned.rotation * Vec3::Y).normalize()
                                };

        
                                let enemy_futur_position = match enemy_futures_positions_by_movement_option.get(&movement_option) {
                                    Some(enemy_futur_position_known) => *enemy_futur_position_known,
                                    None => {
                                        let result = enemy_transform.translation + enemy_new_direction * ((enemy.speed as u32 * move_factor / DODGE_ACCURACY) as f32) * 1.5 * time.delta_seconds();
                                        enemy_futures_positions_by_movement_option.insert(movement_option, result);
                                        result
                                    }
                                };

                                if check_if_collide(&enemy_collider_circle, enemy_futur_position.truncate(), &threat.collider, threat.position.truncate()) {
                                    forbidden_movements.insert(movement_option);
                                } else {
                                    let threat_new_position = threat.position + threat.direction * threat.distance;
                                    let threat_can_collide_with_actual_enemy_position = check_if_collide(&enemy_collider_circle, enemy_transform.translation.truncate(), &threat.collider, threat_new_position.truncate());
                                    let threat_can_collide_with_new_enemy_position = check_if_collide(&enemy_collider_circle, enemy_futur_position.truncate(), &threat.collider, threat_new_position.truncate());

                                    if threat_can_collide_with_actual_enemy_position {
                                        if threat_can_collide_with_new_enemy_position {
                                            let threat_futur_position_in_three_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds() * 3.;
                                            let threat_distance_traveled_by_frame = threat.position.distance(threat_futur_position_in_one_frame);
                                            let threat_distance_remaining_after_three_frame = enemy_transform.translation.distance(threat_futur_position_in_three_frame);
                                            if threat_distance_remaining_after_three_frame < threat_distance_traveled_by_frame {
                                                forbidden_movements.insert(movement_option);
                                            } else {
                                                possible_actions_and_notation.push((action_option, -1));
                                            }
                                        } else {
                                            // let note = move_factor as i32 + {
                                            //     let new_distance = enemy_transform.translation.distance(threat.position);
                                            //     dbg!(threat.distance, new_distance);
                                            //     if threat.distance < new_distance {
                                            //         1
                                            //     } else {
                                            //         0
                                            //     }
                                            // };
                                            possible_actions_and_notation.push((action_option, move_factor as i32));
                                        }
                                    } else {
                                        if threat_can_collide_with_new_enemy_position {
                                            let threat_futur_position_in_three_frame = threat.position + threat.velocity.extend(0.) * time.delta_seconds() * 3.;
                                            let threat_distance_traveled_by_frame = threat.position.distance(threat_futur_position_in_one_frame);
                                            let threat_distance_remaining_after_three_frame = enemy_transform.translation.distance(threat_futur_position_in_three_frame);
                                            if threat_distance_remaining_after_three_frame < threat_distance_traveled_by_frame {
                                                forbidden_movements.insert(movement_option);
                                            } else {
                                                possible_actions_and_notation.push((action_option, -2));
                                            }
                                        } else {
                                            let note = move_factor as i32 + {
                                                let new_distance = enemy_futur_position.distance(threat.position);
                                                dbg!(threat.distance, new_distance);
                                                if threat.distance < new_distance {
                                                    1
                                                } else {
                                                    0
                                                }
                                            };
                                            possible_actions_and_notation.push((action_option, note));
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                ActionOption::Shoot(_, _) => {
                    if threat.threatened_dot_product > 0. {        
                        // ! Vérifier calcul (pour obtenir l'angle, et impact du signe)
                        let angle = get_angle_in_radian_between_two_entities(enemy_transform.rotation, to_threat);
                        if angle.abs() < (ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING[ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING.len() - 1] as f32).to_radians() {
                            let rotation_direction = if angle > 0. {
                                RotationDirection::Clockwise
                            } else {
                                RotationDirection::CounterClockwise
                            };

                            possible_actions_and_notation.push((ActionOption::Shoot((rotation_direction, angle.abs().to_degrees() as u32), threat.distance as u32), 0));
                        }
                    }
                }
            }
        }

        possible_actions_and_notation_for_all_threats.push(possible_actions_and_notation);
    }

    dbg!(possible_actions_and_notation_for_all_threats.clone());

    let final_possible_actions = reduce_possible_actions(possible_actions_and_notation_for_all_threats, forbidden_movements);
    match final_possible_actions {
        (None, None) => dodge_status = DodgeStatus::Impossible,
        (None, Some(ActionOption::Shoot((rotation_direction, angle), _))) => dodge_status = DodgeStatus::CriticalShoot(rotation_direction, angle),
        (None, Some(_)) => panic!(),
        (_, _) =>  ()
    }

    dbg!(final_possible_actions.clone());

    match dodge_status {
        DodgeStatus::Impossible => (),
        DodgeStatus::CriticalShoot(rotation_direction, angle) => {
            if can_shoot {
                critical_shoot(enemy_transform, enemy, enemy_laser_timer, rotation_direction, angle);
            }
        },
        _ => move_enemy(now, time, enemy_transform, enemy, last_dodge_rotation, final_possible_actions.0.unwrap()) // action de filtre puis de réalisation de l'action
    }
}

fn init_forbidden_movements(last_dodge_rotation: &mut LastDodgeRotation) -> HashSet<MovementOption> {
    match last_dodge_rotation.0 {
        None => HashSet::new(),
        Some(rotation) => {
            last_dodge_rotation.0 = None;
            let rotation_to_forbid = match rotation {
                RotationDirection::Clockwise => RotationDirection::CounterClockwise,
                RotationDirection::CounterClockwise => RotationDirection::Clockwise
            };
            
            let all_rotations = ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITHOUT_MOVING
                .into_iter()
                .map(|angle| MovementOption::Rotation(rotation_to_forbid, angle))
                .collect::<Vec<_>>();
            
            let all_move_and_rotations = (1..=DODGE_ACCURACY)
                .into_iter()
                .map(|move_factor| MovementOption::MoveAndRotation(move_factor, rotation_to_forbid, ALLOWED_ROTATION_ANGLE_IN_DEGREE_IN_ONE_FRAME_WITH_MOVING))
                .collect::<Vec<_>>();
            
            all_rotations.into_iter().chain(all_move_and_rotations.into_iter()).collect()
        }
    }
}

fn get_angle_in_radian_between_two_entities(rotation: Quat, direction: Vec3) -> f32 {
    // This gets the forward vector based on rotation
    let direction = direction.normalize();
    let entity_direction = rotation * Vec3::Y;
    let cross_product = entity_direction.cross(direction);
                    
    let dot_product = entity_direction.normalize().dot(direction).clamp(-1., 1.);
    let mut angle = dot_product.acos();

    if cross_product.z < 0.0 { -angle } else { angle }
}

fn reduce_possible_actions(possible_actions_and_notation_for_all_threats: Vec<Vec<(ActionOption, i32)>>, forbidden_movements: HashSet<MovementOption>) -> (Option<ActionOption>, Option<ActionOption>) {
    let mut possible_movement_and_notation = HashSet::new();
    let mut possible_shoot_and_distance = None;


    for (index, possible_actions_and_notation_by_threat) in possible_actions_and_notation_for_all_threats.iter().enumerate() {
        let mut possible_movement_just_added = HashSet::new();

        for (possible_action, notation_action) in possible_actions_and_notation_by_threat {
            match possible_action {
                ActionOption::Movement(movement_option) => {
                    if forbidden_movements.contains(movement_option) {
                        continue;
                    } else {
                        if index == 0 {
                            possible_movement_and_notation.insert((movement_option.clone(), *notation_action));
                            possible_movement_just_added.insert(movement_option.clone());
                        } else {
                            for (movement, notation) in possible_movement_and_notation.clone() {
                                if *movement_option == movement {
                                    possible_movement_and_notation.remove(&(movement, notation));
                                    possible_movement_and_notation.insert((movement, notation + *notation_action));
                                    possible_movement_just_added.insert(movement_option.clone());
                                }
                            }
                        }
                    }
                },
                ActionOption::Shoot(shoot_direction, shoot_ditance) => {
                    match possible_shoot_and_distance {
                        Some((_, distance)) => {
                            if *shoot_ditance < distance {
                                possible_shoot_and_distance = Some((shoot_direction.clone(), *shoot_ditance));
                            }
                        },
                        None => {
                            possible_shoot_and_distance = Some((shoot_direction.clone(), *shoot_ditance));
                        }
                    }
                }
            }
        }

        possible_movement_and_notation = possible_movement_and_notation.into_iter().filter(|(movement, _)| possible_movement_just_added.contains(movement)).collect::<HashSet<_>>();
        
    }

    let final_possible_movement_and_notation = possible_movement_and_notation.iter().max_by_key(|(_, notation)| *notation).map(|(movement, _)| ActionOption::Movement(*movement));
    let final_possible_shoot_and_distance = match possible_shoot_and_distance {
        Some((rotation_direction, distance)) => Some(ActionOption::Shoot(rotation_direction, distance)),
        None => None
    };

    (final_possible_movement_and_notation, final_possible_shoot_and_distance)
}

fn critical_shoot(enemy_transform: &mut Mut<Transform>, enemy: &Enemy, enemy_laser_timer: &mut LaserTimer, rotation_direction: RotationDirection, angle: u32) {                    
    let signed_angle_in_radians = match rotation_direction {
        RotationDirection::Clockwise => (angle as f32).to_radians(),
        RotationDirection::CounterClockwise => (angle as f32).to_radians() * -1.,
    };
                    
    enemy_transform.rotate(Quat::from_rotation_z(signed_angle_in_radians));
    enemy_laser_timer.renew(enemy.shoot_delay_seconds);
    // todo : ajouter event de shoot
    println!("shoot");
}

fn move_enemy(now: Instant, time: &Res<Time>, enemy_transform: &mut Mut<Transform>, enemy: &Enemy, last_dodge_rotation: &mut LastDodgeRotation, action: ActionOption) {
    
    match action {
        ActionOption::Movement(movement) => {
            match movement {
                MovementOption::Stationary => (),
                MovementOption::Move(move_factor) => {
                    let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
                    enemy_transform.translation += enemy_direction * ((enemy.speed as u32 * move_factor / DODGE_ACCURACY) as f32) * 1.5 * time.delta_seconds();
                },
                MovementOption::Rotation(rotation_direction, angle) => {
                    // dbg!(movement);
                    last_dodge_rotation.0 = Some(rotation_direction);
                    
                    let signed_angle_in_radians = match rotation_direction {
                        RotationDirection::Clockwise => (angle as f32).to_radians(),
                        RotationDirection::CounterClockwise => (angle as f32).to_radians() * -1.,
                    };
                                    
                    enemy_transform.rotate(Quat::from_rotation_z(signed_angle_in_radians));
                },
                MovementOption::MoveAndRotation(move_factor, rotation_direction, angle) => {
                    // dbg!(movement);

                    last_dodge_rotation.0 = Some(rotation_direction);

                    let signed_angle_in_radians = match rotation_direction {
                        RotationDirection::Clockwise => (angle as f32).to_radians(),
                        RotationDirection::CounterClockwise => (angle as f32).to_radians() * -1.,
                    };
                                    
                    enemy_transform.rotate(Quat::from_rotation_z(signed_angle_in_radians));
                    let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
                    enemy_transform.translation += enemy_direction * ((enemy.speed as u32 * move_factor / DODGE_ACCURACY) as f32) * 1.5 * time.delta_seconds();
                }
            }
        },
        _ => panic!()
    };
    // println!("Time {:?}", time.delta().as_micros());
    // println!("Instant {:?}", now.elapsed().as_micros());


}







#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;
    use bevy::ecs::system::SystemState;

    fn assert_in_list<T: PartialEq + Debug>(value: T, expected_values: &[T]) {
        if !expected_values.contains(&value) {
            panic!("Value {:?} is not in the expected list {:?}", value, expected_values);
        }
    }

    #[test]
    fn test_reduce_possible_actions() {
        // Given
        struct Case {
            action_options: Vec<Vec<(ActionOption, i32)>>,
            forbiden_movements: HashSet<MovementOption>,
            results: Vec<ActionOption>
        }

        // Case 1 : Highest score without any impact of forbidden movements
        let case_1 = Case {
            action_options: vec![
                vec![
                    (ActionOption::Movement(MovementOption::Stationary), 0),
                    (ActionOption::Movement(MovementOption::Move(1)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 10)), 0),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 40)), 2),
                    (ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10)), 1),
                ]
            ],
            forbiden_movements: vec![
                MovementOption::Rotation(RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 40),
                MovementOption::MoveAndRotation(1, RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 30),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 20)
            ].into_iter().collect::<HashSet<_>>(),
            results: vec![ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 40))]
        };
        

        // Case 2 : Several possible solutions without any impact of forbidden movements
        let case_2 = Case {
            action_options: vec![
                vec![
                    (ActionOption::Movement(MovementOption::Stationary), 0),
                    (ActionOption::Movement(MovementOption::Move(1)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 10)), 0),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)), 1),
                    (ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10)), 1),
                ]
            ],
            forbiden_movements: vec![
                MovementOption::Rotation(RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 40),
                MovementOption::MoveAndRotation(1, RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 30),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 20)
            ].into_iter().collect::<HashSet<_>>(),
            results: vec![
                ActionOption::Movement(MovementOption::Move(1)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)),
                ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10))
            ]
        };
        
        
        // Case 3 : Several possible solutions after highest score is removed by the forbidden movement list
        let case_3 = Case {
            action_options: vec![
                vec![
                    (ActionOption::Movement(MovementOption::Stationary), 0),
                    (ActionOption::Movement(MovementOption::Move(1)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 10)), 0),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 40)), 2),
                    (ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10)), 1),
                ]
            ],
            forbiden_movements: vec![
                MovementOption::Rotation(RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 40),
                MovementOption::MoveAndRotation(1, RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 30),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 20),
                MovementOption::Rotation(RotationDirection::Clockwise, 40)
            ].into_iter().collect::<HashSet<_>>(),
            results: vec![
                ActionOption::Movement(MovementOption::Move(1)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)),
                ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10))
            ]
        };
        

        // Case 4 : Several possible solutions after highest score is removed by the forbidden movement list
        let case_4 = Case {
            action_options: vec![
                vec![
                    (ActionOption::Movement(MovementOption::Stationary), 0),
                    (ActionOption::Movement(MovementOption::Move(1)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 10)), 0),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)), 1),
                    (ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 40)), 2),
                    (ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10)), 1),
                ]
            ],
            forbiden_movements: vec![
                MovementOption::Rotation(RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 40),
                MovementOption::MoveAndRotation(1, RotationDirection::CounterClockwise, 10),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 30),
                MovementOption::Rotation(RotationDirection::CounterClockwise, 20),
                MovementOption::Rotation(RotationDirection::Clockwise, 40)
            ].into_iter().collect::<HashSet<_>>(),
            results: vec![
                ActionOption::Movement(MovementOption::Move(1)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 20)),
                ActionOption::Movement(MovementOption::Rotation(RotationDirection::Clockwise, 30)),
                ActionOption::Movement(MovementOption::MoveAndRotation(1, RotationDirection::Clockwise, 10))
            ]
        };

        // When
        let result_1 = reduce_possible_actions(case_1.action_options, case_1.forbiden_movements);
        let result_2 = reduce_possible_actions(case_2.action_options, case_2.forbiden_movements);
        let result_3 = reduce_possible_actions(case_3.action_options, case_3.forbiden_movements);

        // Then
        assert_in_list(result_1.0.unwrap(), &case_1.results);
        assert_in_list(result_2.0.unwrap(), &case_2.results);
        assert_in_list(result_3.0.unwrap(), &case_3.results);
    }

    // #[test]
    // fn test_reduce_possible_actions() {
    //     // Créez une nouvelle application Bevy pour le test
    //     let mut app = App::build();

    //     // Ajoutez la composante Position et la fonction update_position à l'application
    //     app.add_plugins(MinimalPlugins)
    //         .add_startup_system(setup.system())
    //         .add_system(update_position.system());

    //     // Initialisez l'application
    //     app.update();

    //     // Récupérez le monde de l'application
    //     let world = app.world();

    //     // Récupérez l'entité et la composante Position
    //     let position = world.query::<&Position>().single(&world);

    //     // Vérifiez que la position a été mise à jour correctement
    //     assert_eq!(position.x, 1.0);
    //     assert_eq!(position.y, 1.0);
    // }

    // // Système de configuration pour initialiser l'entité avec la composante Position
    // fn setup(mut commands: Commands) {
    //     commands.spawn().insert(Position { x: 0.0, y: 0.0 });
    // }
}