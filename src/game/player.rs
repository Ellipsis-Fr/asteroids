use std::time::Instant;
use  bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use rand::{random, Rng};
use super::{GameTextures, PLAYER_SIZE, SPRITE_SCALE, components::{Player, Velocity, Movable, Direction, SpriteSize, Laser, LifeTime, RocketDragTimer, RocketFire}, LASER_SIZE };


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
                    player_velocity_event_system,
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
        .insert(Player)
        .insert(SpriteSize::from(PLAYER_SIZE))
        .insert(Velocity::default())
        .insert(Movable { auto_despawn: false })
        .insert(Direction::default());
}

fn player_rotation_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Direction, With<Player>>) {
    if let Ok(mut rotation) = query.get_single_mut() {
        if kb.pressed(KeyCode::ArrowLeft) {
            rotation.rotate(1);
        } else if kb.pressed(KeyCode::ArrowRight) {
            rotation.rotate(-1);
            
        }
    }    
}

fn player_velocity_event_system(
    mut commands: Commands,
    kb: Res<ButtonInput<KeyCode>>,
    game_textures: Res<GameTextures>,
    mut query: Query<(&Transform, &mut Velocity, &mut Direction), With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok((transform, mut velocity, mut rotation)) = query.get_single_mut() {
        if kb.pressed(KeyCode::ArrowUp) {
            velocity.accelerate();
            velocity.calculate_translation(&rotation.rotation_angle_degrees);

            let y_offset = PLAYER_SIZE.0 / 2. * SPRITE_SCALE * -1. + 10.;
            let rocket_fire_translation = calculate_shifted_translation(&transform.translation, rotation.rotation_angle_degrees, y_offset);

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
                        rotation: Quat::from_rotation_z(rotation.rotation_angle_degrees.to_radians())
                    },
                    ..Default::default()
                })
                .insert(RocketFire)
                .insert(LifeTime(Timer::from_seconds(0.05, TimerMode::Once)));
            
            
            if rand::thread_rng().gen::<f32>() > 0.75 {
                let rocket_drag_timer = RocketDragTimer::new(0.25);
                let life_time_in_seconds_for_rocket_drag = rocket_drag_timer.2.duration().as_secs_f32();
                let random_angle = rand::thread_rng().gen_range(-25..=25) as f32;
                
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
                    .insert(Velocity::with_direction(0.25, rotation.rotation_angle_degrees + 180. + random_angle))
                    .insert(Movable { auto_despawn: true })
                    .insert(rocket_drag_timer)
                    .insert(LifeTime(Timer::from_seconds(life_time_in_seconds_for_rocket_drag, TimerMode::Once)));
            }

        } else {
            velocity.stop();
        }
    }    
}

fn player_shooting_system(
    mut commands: Commands,
    time_since_last_shot: Option<ResMut<TimeSinceLastShot>>,
    game_textures: Res<GameTextures>,
    kb: Res<ButtonInput<KeyCode>>,
    query: Query<(&Transform, &mut Direction), With<Player>>
) {
    if let Ok((transform, mut rotation)) = query.get_single() {
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
            let laser_translation = calculate_shifted_translation(&transform.translation, rotation.rotation_angle_degrees, y_offset);

            commands
                .spawn(SpriteBundle {
                    texture: game_textures.laser.clone(),
                    sprite: Sprite {
                        ..Default::default()
                    },
                    transform: Transform {
                        translation: laser_translation,
                        scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                        rotation: Quat::from_rotation_z(rotation.rotation_angle_degrees.to_radians())
                    },
                    ..Default::default()
                })
                .insert(Laser)
                .insert(SpriteSize::from(LASER_SIZE))
                .insert(Velocity::with_direction(1., rotation.rotation_angle_degrees))
                .insert(Movable { auto_despawn: true })
                .insert(LifeTime(Timer::from_seconds(1., TimerMode::Once)));
        }
    }
}

fn calculate_shifted_translation(translation_origin: &Vec3, angle: f32, shift: f32) -> Vec3 {
    let angle_radians = angle.to_radians();

    let x = angle_radians.sin() * shift * -1.;
    let y = angle_radians.cos() * shift;

    Vec3 { x: translation_origin.x + x, y: translation_origin.y + y, z: translation_origin.z }
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