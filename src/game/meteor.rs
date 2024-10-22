use std::f32::consts::TAU;

use bevy::{log::tracing_subscriber::field::debug, prelude::*};
use bevy_rapier2d::{parry::simba::scalar::SupersetOf, prelude::{Collider, ColliderMassProperties, Restitution, RigidBody, Sleeping, Velocity}};
use rand::Rng;

use super::{components::{Direction, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, RocketDragTimer, RocketFire}, wave::Wave, GameTextures, WinSize, BASE_SPEED, LASER_SIZE, METEOR_SIZE, PLAYER_SIZE, SPRITE_SCALE, TIME_STEP };

#[derive(Debug)]
pub struct MeteorDefinition {
    pub weight: f32,
    pub speed: [f32; 2],
    pub kind: u8,
    pub level: u8,
}

#[derive(Debug)]
struct MeteorMapper {
    init_position: Vec3,
    weight: f32,
    linvel: Vec2,
    angvel: f32,
    restitution_coefficient: f32,
    kind: u8,
    level: u8,
}

pub struct MeteorPlugin;

impl Plugin for MeteorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, meteor_spawn_system.run_if(enough_meteors_to_spawn));
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
    let meteor_to_spawn = get_meteor_definition_mapped(&win_size, wave_resource.get_meteors().pop().unwrap());

    let x = rand::thread_rng().gen_range((0.)..win_size.width);
	let y = win_size.height + 50.;

    commands
        .spawn(SpriteBundle {
            texture: game_textures.meteor.clone(),
            sprite: Sprite {
                ..Default::default()
            },
            transform: Transform {
                translation: meteor_to_spawn.init_position,
                scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball(METEOR_SIZE.0 / 2.))
        .insert(ColliderMassProperties::Mass(meteor_to_spawn.weight))
        .insert(Velocity {
            linvel: meteor_to_spawn.linvel,
            angvel: meteor_to_spawn.angvel,
        })
        .insert(Restitution::coefficient(meteor_to_spawn.restitution_coefficient))
        .insert(Sleeping::disabled());
}

fn get_meteor_definition_mapped(win_size: &Res<WinSize>, meteor_definition: MeteorDefinition) -> MeteorMapper {
    MeteorMapper { 
        init_position: Vec3 { 
            x: win_size.width * rand::thread_rng().gen_range(-1.0..=1.0),
            y: win_size.height * rand::thread_rng().gen_range(-1.0..=1.0),
            z: 10. 
        },
        weight: meteor_definition.weight,
        linvel: Vec2 { 
            x: rand::thread_rng().gen_range((meteor_definition.speed[0] * TIME_STEP * BASE_SPEED)..=((meteor_definition.speed[1] * TIME_STEP * BASE_SPEED))) * rand::thread_rng().gen_range(-1.0..=1.0),
            y: rand::thread_rng().gen_range((meteor_definition.speed[0] * TIME_STEP * BASE_SPEED)..=((meteor_definition.speed[1] * TIME_STEP * BASE_SPEED))) * rand::thread_rng().gen_range(-1.0..=1.0)
        },
        angvel: rand::thread_rng().gen_range((0.)..TAU),
        restitution_coefficient: 1.,
        kind: meteor_definition.kind,
        level: meteor_definition.level
    }
}