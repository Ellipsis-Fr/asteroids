use std::f32::consts::TAU;

use bevy::{app::{App, Plugin, Update}, asset::Assets, color::Color, ecs::{event::EventReader, system::{Commands, ResMut}}, math::{primitives::Rectangle, Vec2, Vec3}, render::mesh::Mesh, sprite::{ColorMaterial, MaterialMesh2dBundle}, time::{Timer, TimerMode}, transform::components::Transform, utils::default};
use bevy_rapier2d::prelude::{RigidBody, Velocity};
use rand::{random, Rng};

use super::{components::{Fragment, LifeTime}, events::FragmentEvent};

const MAX_NUMBER_OF_PARTS: u16 = 10;
const AVG_SPEED_OF_PARTS: i16 = 100;

pub struct FragmentPlugin;

impl Plugin for FragmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fragment_system);
    }
}

fn fragment_system(
    mut commands: Commands,
    mut fragment_events: EventReader<FragmentEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    for (index, fragment_event) in fragment_events.read().enumerate() {
        let translation = fragment_event.0;

        for i in (1..=MAX_NUMBER_OF_PARTS).rev() {
            commands
                .spawn(MaterialMesh2dBundle {
                    mesh: meshes.add(Rectangle::new(2., 2.)).into(),
                    material: materials.add(ColorMaterial::from(get_color())),
                    transform: Transform {
                        translation,
                        scale: Vec3::new(1., 1., 1.),
                        ..default()
                    },
                    ..default()
                })
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Velocity::linear(calculate_velocity(i)))
                .insert(Fragment)
                .insert(LifeTime(Timer::from_seconds(0.5, TimerMode::Once)));
        }
    }
}

fn get_color() -> Color {
    let colors = vec![
        Color::xyz(1., 0.64, 0.), // Orange
        Color::xyz(1., 0., 0.), // Red
        Color::xyz(1., 1., 0.), // Yellow
        Color::xyz(0.65, 0.16, 0.16), // Brown
    ];

    colors[rand::thread_rng().gen_range(0..=3)].clone()
}

fn calculate_velocity(parts_number: u16) -> Vec2 {
    let actual_speed = AVG_SPEED_OF_PARTS + rand::thread_rng().gen_range(-50..=50);
    
    let mut direction = get_direction(parts_number);
    direction.normalize_or_zero() * actual_speed as f32
}

fn get_direction(parts_number: u16) -> Vec2 {
    let angle_in_radians = ((TAU.to_degrees() / MAX_NUMBER_OF_PARTS as f32) * parts_number as f32 + rand::thread_rng().gen_range(-5..=5) as f32).to_radians();
    Vec2 { x: angle_in_radians.sin(), y: angle_in_radians.cos() }
}