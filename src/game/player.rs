use crate::game::{GameTextures, WinSize, PLAYER_SIZE, SPRITE_SCALE, components::{Player, Velocity, Movable, Rotation, FromPlayer, SpriteSize, Laser}, TIME_STEP, BASE_SPEED };
use  bevy::prelude::*;

// region:    --- Constants

const VELOCITY_MAX: f32 = 5.;
// endregion: --- Constants

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PostStartup, player_spawn_system)
            .add_systems(Update, (player_velocity_event_system, player_rotation_event_system));
    }
}

fn player_spawn_system(
	mut commands: Commands,
	win_size: Res<WinSize>,
	game_textures: Res<GameTextures>,
) {
	// add player
	let bottom = - win_size.height / 2.;
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
        .insert(Rotation::default());
}

fn player_velocity_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity, With<Player>>) {
    if let Ok(mut velocity) = query.get_single_mut() {
        velocity.up = if kb.pressed(KeyCode::ArrowUp) {
            1.
        } else {
            0.
        };
    }    
}

fn player_rotation_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Rotation, With<Player>>) {
    if let Ok(mut rotation) = query.get_single_mut() {
        if kb.pressed(KeyCode::ArrowLeft) {
            rotation.rotate(1);
        } else if kb.pressed(KeyCode::ArrowRight) {
            rotation.rotate(-1);
            
        }
    }    
}