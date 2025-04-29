use std::{collections::{BTreeMap, HashMap}, ops::Sub, time::Instant};

use bevy::{prelude::*, utils::hashbrown::HashSet};
use bevy_rapier2d::{na::Rotation, prelude::*};

use crate::game::collision::check_if_collide;

use super::{collision, components::{AIState, DetectionSensor, Enemy, EnemyState, FakeEntities, LaserTimer, Meteor, Player}, screen_overflow, wave::Wave, GameTextures, WinSize, ENEMY_SIZE, SPRITE_SCALE};

const DODGE_ACCURACY: u32 = 4; // enemy movement decomposition number
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
    Shoot,
}

impl ActionOption {
    fn all_variants() -> Vec<ActionOption> {
        let movement_options = MovementOption::all_variants()
            .into_iter()
            .map(ActionOption::Movement)
            .collect::<Vec<_>>();
        vec![movement_options, vec![ActionOption::Shoot]].concat()
    }
}


#[derive(Debug, Eq, PartialEq, Hash, Clone)]
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
                    direction: projectile_direction,
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
    let enemy_direction = enemy_transform.rotation * Vec3::Y;
    let cross_product = enemy_direction.cross(direction);
                    
    let dot_product = enemy_direction.normalize().dot(direction).clamp(-1., 1.);
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
    let mut enemy_futures_positions_by_movement_option = HashMap::new();

    let can_shoot = enemy_laser_timer.0.finished();
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
                        DodgeStatus::CriticalShoot(RotationDirection::Clockwise, angle as u32)
                    } else {
                        DodgeStatus::CriticalShoot(RotationDirection::CounterClockwise, angle.abs() as u32)
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

        let mut possible_movements_and_notation = Vec::new(); // liste de listes de mouvements possible associer à des notes : Vec<Vec<(MovementOption, i8)>>
        
        // Iterer sur les deplacement possibles en sautant ceux placés dans la liste des dplcts impossibles
        for movement_option in MovementOption::all_variants() {
            match movement_option {
                MovementOption::Stationary => {
                    if !forbidden_movements.contains(&MovementOption::Stationary) && dodge_status == DodgeStatus::Free {
                        possible_movements_and_notation.push((movement_option, 0));
                    }
                },
                MovementOption::Move(move_factor) => {
                    if !forbidden_movements.contains(&MovementOption::Move(move_factor)) {
                        let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();

                        let enemy_futur_position = match enemy_futures_positions_by_movement_option.get(&MovementOption::Move(move_factor)) {
                            Some(enemy_futur_position_known) => *enemy_futur_position_known,
                            None => {
                                let result = enemy_transform.translation + enemy_direction * ((enemy.speed as u32 * move_factor / DODGE_ACCURACY) as f32) * time.delta_seconds();
                                enemy_futures_positions_by_movement_option.insert(MovementOption::Move(move_factor), result);
                                result
                            }
                        };

                        if check_if_collide(&enemy_collider_circle, enemy_futur_position.truncate(), &threat.collider, threat.position.truncate()) {
                            forbidden_movements.insert(MovementOption::Move(move_factor));
                            break;
                        } else {
                            let mut note = move_factor;
                            
                            if threat.distance < enemy_futur_position.distance(threat.position) {
                                note += 1;
                            }

                            if threat.threat_dot_product > threat.direction.dot((threat.position - enemy_futur_position).normalize()).clamp(-1., 1.) {
                                note += 1;
                            }

                            possible_movements_and_notation.push((movement_option, note));
                        }
                    }
                },
                MovementOption::Rotation(rotation_direction, angle_degree) => {
                    // let enemy_direction = (enemy_transform.rotation * Vec3::Y).normalize();
                    // threatened_dot_product: enemy_direction.dot(to_projectile.normalize()).clamp(-1., 1.),

                    if threat.threat_dot_product <= 0. {
                        possible_movements_and_notation.push((movement_option, 0));
                    } else {

                    }
                    
                    let enemy_actual_direction = (enemy_transform.rotation * Vec3::Y).normalize();

                    // todo : evaluation va porter sur l'angle Bêta (threat.threatened_dot_product) donc entre direction du vaisseau et to_threat
                    // si threat.dot_product <= 0 (angle >= 90°) alors ça n'est pas grave si l'angle Bêta diminue et les points sont égales à l'angle / 10 
                    // par contre si threat.dot_product > 0 alors l'ajouter mais avec 0 points 
                    // ajout car cf.F2 M3 est à 50° par rapport à direction actuelle du vaisseau mais va gagner jusqu'à 4 points vis à vis d'M1 et c'est la meilleure solution

                    // let mut enemy_transform_cloned = enemy_transform.clone();
                    // enemy_transform_cloned.rotate(Quat::from_rotation_z(10_f32.to_radians()));
                    // let enemy_direction = (enemy_transform_cloned.rotation * Vec3::Y).normalize();
                    // let enemy_dot_product = enemy_direction.dot(to_projectile.normalize()).clamp(-1., 1.);
                },
                MovementOption::MoveAndRotation(move_factor, rotation_direction, angle_degree) => todo!(),
            }
        }

        // todo: certainement voir pour ajouter la possibilité d'un shoot pour chaque menace pour que s'il n'y a aucune possibilité de mouvement après avoir itéré sur 
        // tous les projectiles on shoot le projectile à la fois le plus proche et le plus accessible, car sinon c'est que le vaisseau risque de ne plus du tout avoir de porte de sortie
        possible_movements_and_notation_for_all_threats.push(possible_movements_and_notation);
    }

    // todo: comme la liste de mouvement interdit est remplie au fur et à mesure il serait bon de réaliser le filtre des actions par menace à ce moment là
    // sur chaque liste de possible_movements_and_notation_for_all_threats on y soustrait les éléments présent dans forbidden_movements
    // puis on crée une liste unique contenant les quelques options de mouvement restantes, c-à-d. celles communes à l'ensemble des menaces
    // alors si la liste qui en ressort est vide on passe sur l'option de criticalShoot <-- Pour se faire la liste sera forcéent non vide car il aura fallu y persister pour chaque menace les infos de shoot si ceux-ci sont possible
    // et si la liste est vraiment vide (shoot compris) on passe en dodgeStatus::Impossible 
    // si non vide on envoie ces infos à la fonction de réalisation de l'action qui sélectionnera celle avec la meilleur note, si égalité on prend au pif

    match dodge_status {
        DodgeStatus::Impossible => (),
        DodgeStatus::CriticalShoot(rotation_direction, angle) => todo!(), // action de tire sur cible
        _ => todo!() // action de filtre puis de réalisation de l'action
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