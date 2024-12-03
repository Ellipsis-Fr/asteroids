use std::f32::consts::PI;

use bevy::{prelude::{Component, Vec2, Vec3}, reflect::Reflect, time::{Timer, TimerMode}};

use crate::game::{BASE_SPEED, TIME_STEP};

use rand::{random, Rng};

// region:    --- Common Components
const MAX_ACCELERATION: f32 = 0.5;

#[derive(Component)]
pub struct Acceleration{
    pub acceleration: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Acceleration {
    fn default() -> Self {
        Self { acceleration: 0., x: 0., y: 0. }
    }
}

impl Acceleration {
    pub fn accelerate(&mut self) {
        self.acceleration += if self.acceleration < MAX_ACCELERATION { 0.001 } else { 0. };
    }

    pub fn stop(&mut self) {
        self.acceleration = 0.;
    }

    pub fn calculate_translation(&mut self, rotation_angle_degrees: &f32) {
        let angle_radians = rotation_angle_degrees.to_radians();
        self.x += angle_radians.sin() * self.acceleration * -1.;
        self.y += angle_radians.cos() * self.acceleration;

        self.correct_max_acceleration();
    }
        
    fn correct_max_acceleration(&mut self) {
        if self.x > MAX_ACCELERATION {
            self.x = MAX_ACCELERATION;
        } else if self.x < -MAX_ACCELERATION {
            self.x = -MAX_ACCELERATION;
        }
        if self.y > MAX_ACCELERATION {
            self.y = MAX_ACCELERATION;
        } else if self.y < -MAX_ACCELERATION {
            self.y = -MAX_ACCELERATION;
        }
    }
}

const MAX_ANGLE_VALUES: (f32, f32) = (0., 360.);

#[derive(Component)]
pub struct Direction {
    pub rotation_angle_degrees: f32
}

impl Default for Direction {
    fn default() -> Self {
        Self { rotation_angle_degrees: 0. }
    }
}

impl Direction {
    pub fn rotate(&mut self, rotation: f32) {
        self.rotation_angle_degrees += rotation * TIME_STEP * BASE_SPEED;
        self.correct_angle();
    }

    fn correct_angle(&mut self) {
        if self.rotation_angle_degrees < MAX_ANGLE_VALUES.0 {
            self.rotation_angle_degrees = MAX_ANGLE_VALUES.1 - self.rotation_angle_degrees.abs();
        } else if self.rotation_angle_degrees >= MAX_ANGLE_VALUES.1 {
            self.rotation_angle_degrees = self.rotation_angle_degrees - MAX_ANGLE_VALUES.1;
        }
    }
}

#[derive(Component)]
pub struct Laser;

#[derive(Component)]
pub struct LaserTimer(pub Timer);

impl Default for LaserTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1., TimerMode::Once))
    }
}
// endregion: --- Common Components

// region:    --- Player Component
#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct FromPlayer;
// endregion: --- Player Component

// region:    --- Enemy Component
#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct FromEnemy;
// endregion: --- Enemy Component

// region:    --- Explosion Component
#[derive(Component)]
pub struct Explosion;

#[derive(Component)]
pub struct ExplosionToSpawn(pub Vec3);

#[derive(Component)]
pub struct ExplosionTimer(pub Timer);

impl Default for ExplosionTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.05, TimerMode::Repeating))
    }
}
// endregion: --- Explosion Component

#[derive(Component)]
pub struct RocketFire;

#[derive(Component)]
pub struct RocketDrag;

#[derive(Component)]
pub struct RocketDragTimer(pub Timer, pub Timer, pub Timer);

impl RocketDragTimer {
    pub fn new(mut factor: f32) -> Self {
        factor += rand::thread_rng().gen::<f32>();
        
        let duration_1_in_seconds = 1. * factor;
        let duration_2_in_seconds = duration_1_in_seconds + 0.5 * factor;
        let duration_3_in_seconds = duration_2_in_seconds + 0.25 * factor;
        
        Self(
            Timer::from_seconds(duration_1_in_seconds, TimerMode::Once),
            Timer::from_seconds(duration_2_in_seconds, TimerMode::Once),
            Timer::from_seconds(duration_3_in_seconds, TimerMode::Once)
        )
    }
}

#[derive(Component)]
pub struct LifeTime(pub Timer);


// region:    --- Meteor Component
#[derive(Component)]
pub struct Meteor;

#[derive(Component)]
pub struct Weight(i32);

#[derive(Component)]
pub struct MeteorType(u8);

#[derive(Component, Reflect)]
pub struct MeteorLevel(pub u8);

#[derive(Component)]
pub struct MeteorState(u8);
// endregion: --- Meteor Component