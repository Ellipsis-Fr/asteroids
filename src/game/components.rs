use std::f32::consts::PI;

use bevy::{prelude::{Component, Vec2, Vec3}, time::{Timer, TimerMode}};

use crate::game::{BASE_SPEED, TIME_STEP};

// region:    --- Common Components
const MAX_VELOCITY: f32 = 0.5;

#[derive(Component)]
pub struct Velocity {
    pub acceleration: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Velocity {
    fn default() -> Self {
        Self { acceleration: 0., x: 0., y: 0. }
    }
}

impl Velocity {

    pub fn with_direction(acceleration: f32, angle_degrees: f32) -> Self {
        let angle_radians = angle_degrees.to_radians();
        Self { acceleration, x: angle_radians.sin() * acceleration * -1., y: angle_radians.cos() * acceleration }
    }

    pub fn accelerate(&mut self) {
        self.acceleration += if self.acceleration < MAX_VELOCITY {
            0.001
        } else {
            0.
        };
    }

    pub fn decelerate(&mut self) {
        self.acceleration = 0.;
    }

    pub fn calculate_translation(&mut self, rotation_angle_degrees: &f32) {
        let angle_radians = rotation_angle_degrees.to_radians();
        self.x += angle_radians.sin() * self.acceleration * -1.;
        self.y += angle_radians.cos() * self.acceleration;

        self.correct_max_velocity();
    }
        
    fn correct_max_velocity(&mut self) {
        if self.x > MAX_VELOCITY {
            self.x = MAX_VELOCITY;
        } else if self.x < -MAX_VELOCITY {
            self.x = -MAX_VELOCITY;
        }
        if self.y > MAX_VELOCITY {
            self.y = MAX_VELOCITY;
        } else if self.y < -MAX_VELOCITY {
            self.y = -MAX_VELOCITY;
        }
    }
}

#[derive(Component)]
pub struct Movable {
    pub auto_despawn: bool,
}

const MAX_ANGLE_VALUES: (f32, f32) = (0., 360.);

#[derive(Component)]
pub struct Rotation {
    pub rotation_angle_degrees: f32
}

impl Default for Rotation {
    fn default() -> Self {
        Self { rotation_angle_degrees: 0. }
    }
}

impl Rotation {
    pub fn rotate(&mut self, rotation: i32) {
        self.rotation_angle_degrees += rotation as f32 * TIME_STEP * BASE_SPEED;
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

#[derive(Component)]
pub struct SpriteSize(pub Vec2);

impl From<(f32, f32)> for SpriteSize {
    fn from(val: (f32, f32)) -> Self {
        SpriteSize(Vec2::new(val.0, val.1))
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
pub struct RocketDragTimer {
    pub span_life: Timer,
    pub cycle: Timer,
}

impl Default for RocketDragTimer {
    fn default() -> Self {
        Self { span_life: Timer::from_seconds(1., TimerMode::Once), cycle: Timer::from_seconds(0.25, TimerMode::Repeating) }
    }
}

#[derive(Component)]
pub struct LifeTime(pub Timer);