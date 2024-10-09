use bevy::{log::tracing_subscriber::field::debug, prelude::*};
use bevy_rapier2d::parry::simba::scalar::SupersetOf;
use rand::Rng;

use super::{components::{Direction, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, Movable, RocketDragTimer, RocketFire, SpriteSize, Velocity}, wave::Wave, GameTextures, WinSize, BASE_SPEED, LASER_SIZE, METEOR_SIZE, PLAYER_SIZE, SPRITE_SCALE, TIME_STEP };

#[derive(Debug)]
pub struct MeteorDefinition {
    pub weight: i32,
    pub speed: [f32; 2],
    pub kind: u8,
    pub level: u8,
}

pub struct MeteorPlugin;

impl Plugin for MeteorPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (meteor_spawn_system.run_if(enough_meteors_to_spawn), rotate_meteor_system));
    }
}

fn enough_meteors_to_spawn(wave_resource: Res<Wave>) -> bool {
    wave_resource.has_meteors()
}

fn meteor_spawn_system(
    mut commands: Commands,
    win_size: Res<WinSize>,
    mut wave_resource: ResMut<Wave>,
    game_textures: Res<GameTextures>
) {
    let meteor_to_spawn = wave_resource.get_meteors().pop().unwrap();

	let up = win_size.height + 50.;
	
    let direction = rand::thread_rng().gen_range((0.)..=360.);
    
    commands
        .spawn(SpriteBundle {
            texture: game_textures.meteor.clone(),
            sprite: Sprite {
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3 { x: 0., y: up, z: 10. },
                scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Meteor { rotation_speed: f32::to_radians(rand::thread_rng().gen_range((25.)..=180.))})
        .insert(SpriteSize::from(METEOR_SIZE))
        .insert(Velocity::with_direction(rand::thread_rng().gen_range((meteor_to_spawn.speed[0])..=meteor_to_spawn.speed[1]), direction))
        .insert(Movable { auto_despawn: false })
        .insert(Direction { rotation_angle_degrees: direction });
}

fn rotate_meteor_system(time: Res<Time>, mut query: Query<(&Meteor, &mut Transform)>) {
    for (meteor, mut transform) in query.iter_mut() {
        transform.rotate_z(meteor.rotation_speed * time.delta_seconds());
    }
}