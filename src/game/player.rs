use std::{f32::consts::PI, time::Instant};
use  bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::{na::Translation, prelude::{ActiveEvents, Collider, CollisionGroups, Group, KinematicCharacterController, RigidBody, Sensor, Velocity}};
use rand::{random, Rng};
use super::{components::{Acceleration, Direction, Laser, LifeTime, Player, RocketDragTimer, RocketFire}, GameTextures, WinSize, BASE_SPEED, LASER_SIZE, PLAYER_SIZE, SPRITE_SCALE, TIME_STEP };


// region:    --- Constants

const LASER_COOLDOWN: f32 = 0.25;
// endregion: --- Constants

// region:    --- Resources
#[derive(Resource)]
pub struct TimeSinceLastShot {
    pub time: Instant,
}
// endregion: --- Resources

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PostStartup, player_spawn_system)
            .add_systems(Update,
        (
                    player_rotation_event_system,
                    player_acceleration_event_system,
                    move_player_system,
                    propulsion_effect_system,
                    edit_rocket_drag_system,
                    player_shooting_system,
                    rotate_player_system,
                ).chain()
            );
    }
}

fn player_spawn_system(mut commands: Commands, game_textures: Res<GameTextures>) {
	commands
        .spawn(SpriteBundle {
            texture: game_textures.player.clone(),
            sprite: Sprite {
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3 { x: 0., y: 0., z: 10. },
                scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(KinematicCharacterController::default())
        .insert(Player)
        .insert(Acceleration::default())
        .insert(Collider::cuboid(PLAYER_SIZE.0 / 2., PLAYER_SIZE.1 / 2.))
        .insert(Direction::default());
}

fn player_rotation_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<(&mut Acceleration, &mut Direction), With<Player>>) {
    if let Ok((mut acceleration, mut rotation)) = query.get_single_mut() {
        if kb.pressed(KeyCode::ArrowLeft) {
            rotation.rotate(0.5);
            acceleration.stop();
        } else if kb.pressed(KeyCode::ArrowRight) {
            rotation.rotate(-0.5);
            acceleration.stop();
        }
    }    
}

fn player_acceleration_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<(&Transform, &mut Acceleration, &Direction), With<Player>>) {
    if let Ok((transform, mut acceleration, direction)) = query.get_single_mut() {
        if kb.pressed(KeyCode::ArrowUp) {
            acceleration.accelerate();
            acceleration.calculate_translation(&direction.rotation_angle_degrees);
        } else {
            acceleration.stop();
        }
    }    
}

fn move_player_system(mut query: Query<(&mut KinematicCharacterController, &Acceleration), With<Player>>) {
    if let Ok((mut controller, acceleration)) = query.get_single_mut() {
        let mut translation = &mut controller.translation.unwrap_or_default();
        translation.x += acceleration.x * TIME_STEP * BASE_SPEED;
        translation.y += acceleration.y * TIME_STEP * BASE_SPEED;
        controller.translation = Some(*translation);
    }
}
fn propulsion_effect_system(
    mut commands: Commands,
    game_textures: Res<GameTextures>,
    query: Query<(&Transform, &Acceleration, &Direction), With<Player>>, // normalement accelation ne sera plus utile, un run_if vérifiera qu'il y a eu ou non acceleration
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok((transform, acceleration, direction)) = query.get_single() {
        if acceleration.acceleration == 0. {
            return;
        }
        let y_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE * -1. + 10.;
        let rocket_fire_translation = calculate_translation(
            Vec2::new(transform.translation.x, transform.translation.y),
            direction.rotation_angle_degrees.to_radians(),
            y_offset
        ).extend(0.);

        // Inserer image de feu et drag
        commands
            .spawn(SpriteBundle {
                texture: game_textures.rocket_fire.clone(),
                sprite: Sprite {
                    ..Default::default()
                },
                transform: Transform {
                    translation: rocket_fire_translation,
                    scale: Vec3::new(0.01, 0.01, 1.),
                    rotation: Quat::from_rotation_z(direction.rotation_angle_degrees.to_radians())
                },
                ..Default::default()
            })
            .insert(RocketFire)
            .insert(LifeTime(Timer::from_seconds(0.05, TimerMode::Once)));
        
        
        if rand::thread_rng().gen::<f32>() > 0.75 {
            let rocket_drag_timer = RocketDragTimer::new(0.25);
            let life_time_in_seconds_for_rocket_drag = rocket_drag_timer.2.duration().as_secs_f32();
            let random_angle = rand::thread_rng().gen_range(-25..=25) as f32;
            let random_angvel = rand::thread_rng().gen_range((0.)..PI);
            
            commands
                .spawn(MaterialMesh2dBundle {
                    mesh: meshes.add(Rectangle::new(2., 2.)).into(),
                    material: materials.add(ColorMaterial::from(Color::xyz(1., 0., 0.))),
                    transform: Transform {
                        translation: rocket_fire_translation,
                        scale: Vec3::new(1., 1., 1.),
                        ..Default::default()
                    },
                    ..default()
                })
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Velocity { linvel: calculate_velocity(Vec2::ZERO, (direction.rotation_angle_degrees + 180. + random_angle).to_radians(), 100.), angvel: random_angvel })
                .insert(rocket_drag_timer)
                .insert(LifeTime(Timer::from_seconds(life_time_in_seconds_for_rocket_drag, TimerMode::Once)));
        }
    }
}

fn player_shooting_system(
    mut commands: Commands,
    time_since_last_shot: Option<ResMut<TimeSinceLastShot>>,
    game_textures: Res<GameTextures>,
    kb: Res<ButtonInput<KeyCode>>,
    query: Query<(&Transform, &Acceleration, &Direction), With<Player>>
) {
    if let Ok((transform, acceleration, direction)) = query.get_single() {
        if kb.just_pressed(KeyCode::Space) {
            if let Some(mut time_since_last_shot) = time_since_last_shot {
                if time_since_last_shot.time.elapsed().as_secs_f32() < LASER_COOLDOWN {
                    return;
                }
                time_since_last_shot.time = Instant::now();
            } else {
                commands.insert_resource(TimeSinceLastShot { time: Instant::now() });
            }

            let y_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE;
            let laser_translation = calculate_translation(
                Vec2::new(transform.translation.x, transform.translation.y),
                direction.rotation_angle_degrees.to_radians(),
                y_offset
            ).extend(0.);
            
            commands
                .spawn(SpriteBundle {
                    texture: game_textures.laser.clone(),
                    sprite: Sprite {
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: laser_translation,
                        scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                        rotation: Quat::from_rotation_z(direction.rotation_angle_degrees.to_radians())
                    },
                    ..Default::default()
                })
                .insert(Laser)
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Collider::capsule(Vec2 { x: 0., y: 0. }, Vec2 { x: 0., y: LASER_SIZE.1 / 2. }, LASER_SIZE.0 / 2.))
                .insert(ActiveEvents::COLLISION_EVENTS)
                .insert(Velocity::linear(calculate_velocity(Vec2::new(acceleration.x, acceleration.y), direction.rotation_angle_degrees.to_radians(), 500.)))
                .insert(LifeTime(Timer::from_seconds(1., TimerMode::Once)));
        }
    }
}

fn calculate_translation(mut vec2: Vec2, angle_radians: f32, shift: f32) -> Vec2 {
    vec2.x += angle_radians.sin() * shift * -1.;
    vec2.y += angle_radians.cos() * shift;

    vec2
}

fn calculate_velocity(vec2: Vec2, angle_radians: f32, boost: f32) -> Vec2 {
    let Vec2 { mut x, mut y } = calculate_translation(Vec2::ZERO, angle_radians, boost);

    let extra_speed = |m: &mut f32, n: f32| {
        if (*m < 0. && n < 0.) || (*m > 0. && n > 0.) {
            *m += n * TIME_STEP * BASE_SPEED;
        }
    };

    extra_speed(&mut x, vec2.x);
    extra_speed(&mut y, vec2.y);
    
    Vec2 { x, y }
}

fn rotate_player_system(mut query: Query<(&mut Transform, &Direction), With<Player>>) {
	if let Ok((mut transform, rotation)) = query.get_single_mut() {
		transform.rotation = Quat::from_rotation_z(rotation.rotation_angle_degrees.to_radians());
	}
}

fn edit_rocket_drag_system(
	time: Res<Time>,
	mut materials: ResMut<Assets<ColorMaterial>>,
	mut query: Query<(&Handle<ColorMaterial>, &mut RocketDragTimer)>
) {
	for (material_handle, mut rocket_drag_timer) in query.iter_mut() {
		
		let mut rocket_drag_has_color_changed = |timer: &mut Timer, color: Color| -> bool {
			if !timer.finished() {
				timer.tick(time.delta());
				
				if timer.just_finished() {
					if let Some(material) = materials.get_mut(material_handle) {
						material.color = color;
					}

					return true;
				}

				return false;
			}

			false
		};

		if !rocket_drag_has_color_changed(&mut rocket_drag_timer.0, Color::srgb(1., 0.5, 0.)) {
			rocket_drag_has_color_changed(&mut rocket_drag_timer.1, Color::srgb(1., 1., 0.));
		}
	}
}