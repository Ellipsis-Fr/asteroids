use std::f32::consts::PI;

use bevy::{prelude::{Component, Vec2, Vec3}, time::{Timer, TimerMode}};

use crate::game::{BASE_SPEED, TIME_STEP};

// region:    --- Common Components
#[derive(Component)]
pub struct Velocity {
    pub up: f32
}

impl Default for Velocity {
    fn default() -> Self {
        Self { up: 0. }
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