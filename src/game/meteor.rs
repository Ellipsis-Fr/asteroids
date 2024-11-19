use std::f32::consts::{PI, TAU};

use bevy::{log::tracing_subscriber::field::debug, prelude::*};
use bevy_rapier2d::{parry::simba::scalar::SupersetOf, prelude::{Collider, ColliderMassProperties, CollisionGroups, ExternalForce, Group, Restitution, RigidBody, Sleeping, Velocity}};
use rand::Rng;

use crate::game::meteor;

use super::{components::{Direction, FromPlayer, Laser, LaserTimer, LifeTime, Meteor, MeteorLevel, RocketDragTimer, RocketFire}, wave::Wave, DestroyedMeteors, GameTextures, WinSize, BASE_SPEED, LASER_SIZE, METEOR_SIZE, PLAYER_SIZE, SPRITE_SCALE, TIME_STEP };

const CHILDREN_METEORS_COUNTER: i32 = 2;

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
        app.add_systems(Update, (
            meteor_spawn_system.run_if(enough_meteors_to_spawn),
            child_meteor_spawn_system.run_if(meteors_destroyed)
        ));
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
    spawn_meteor(&mut commands, &game_textures, meteor_to_spawn);
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
        angvel: rand::thread_rng().gen_range((0.)..PI),
        restitution_coefficient: 1.,
        kind: meteor_definition.kind,
        level: meteor_definition.level
    }
}

fn meteors_destroyed(destroyed_meteors: Res<DestroyedMeteors>) -> bool {
    !destroyed_meteors.0.is_empty()
}

fn child_meteor_spawn_system(mut commands: Commands, game_textures: Res<GameTextures>, mut destroyed_meteors: ResMut<DestroyedMeteors>) {
    let meteors = std::mem::take(&mut destroyed_meteors.0);
    
    for (meteor_definition, translation) in meteors {
        let meteors_to_spawn = get_meteors(translation, meteor_definition);

        for meteor in meteors_to_spawn {
            spawn_meteor(&mut commands, &game_textures, meteor);
        }
    }
}

fn spawn_meteor(commands: &mut Commands, game_textures: &Res<GameTextures>, meteor: MeteorMapper) {
    commands
        .spawn(SpriteBundle {
            texture: game_textures.meteor.clone(),
            sprite: Sprite {
                ..Default::default()
            },
            transform: Transform {
                translation: meteor.init_position,
                scale: Vec3::new(SPRITE_SCALE / meteor.level as f32, SPRITE_SCALE / meteor.level as f32, 1.),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Meteor)
        .insert(MeteorLevel(meteor.level))
        .insert(RigidBody::Dynamic)
        .insert(Collider::ball((METEOR_SIZE.0 / 2.)))
        .insert(ColliderMassProperties::Mass(meteor.weight))
        .insert(Velocity {
            linvel: meteor.linvel,
            angvel: meteor.angvel,
        })
        .insert(Restitution::coefficient(meteor.restitution_coefficient))
        .insert(ExternalForce::default())
        .insert(Sleeping::disabled());
}

fn get_meteors(translation: Vec3, meteor_definition: MeteorDefinition) -> Vec<MeteorMapper> {
    let mut meteors = Vec::new();

    for count in 0..CHILDREN_METEORS_COUNTER {
        let new_direction = get_new_meteor_direction(&translation, &meteor_definition.speed, count as f32);
        
        let init_position = get_init_position(&translation, new_direction.clone().extend(0.));

        let meteor_mapper = MeteorMapper { 
            init_position,
            weight: meteor_definition.weight / 2.,
            linvel: new_direction * 0.5,
            angvel: rand::thread_rng().gen_range((0.)..PI),
            restitution_coefficient: 1.,
            kind: meteor_definition.kind,
            level: meteor_definition.level + 1
        };

        meteors.push(meteor_mapper);
    }

    meteors
}

fn get_new_meteor_direction(translation: &Vec3, actual_meteor_direction: &[f32; 2], count: f32) -> Vec2 {
    const ANGLE_OFFSET_W_CURRENT_DIRECTION: f32 = PI / 2.0;
    const ANGLE_OFFSET_BTW_NEW_DIRECTIONS: f32 = PI;

    let mut new_direction = Vec2::from_array(*actual_meteor_direction);
    new_direction.x = (new_direction.x - translation.x) * (ANGLE_OFFSET_W_CURRENT_DIRECTION + ANGLE_OFFSET_BTW_NEW_DIRECTIONS * count).cos() - (new_direction.y - translation.y) * (ANGLE_OFFSET_W_CURRENT_DIRECTION + ANGLE_OFFSET_BTW_NEW_DIRECTIONS * count).sin() + new_direction.x;
    new_direction.y = (new_direction.x - translation.x) * (ANGLE_OFFSET_W_CURRENT_DIRECTION + ANGLE_OFFSET_BTW_NEW_DIRECTIONS * count).sin() - (new_direction.y - translation.y) * (ANGLE_OFFSET_W_CURRENT_DIRECTION + ANGLE_OFFSET_BTW_NEW_DIRECTIONS * count).cos() + new_direction.y;

    new_direction
}

fn get_init_position(translation: &Vec3, new_direction: Vec3) -> Vec3 {
    translation.clone() + (new_direction.normalize() * Vec3 { x: 10., y: 10., z: 1. })
}