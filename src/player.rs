use crate::{GameTextures, WinSize, PLAYER_SIZE, SPRITE_SCALE, components::{Player, Velocity, Movable, FromPlayer, SpriteSize, Laser}, TIME_STEP, BASE_SPEED };
use  bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PostStartup, player_spawn_system)
            .add_systems(Update, player_keyboard_event_system);
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
		transform: Transform {
			translation: Vec3 { x: 0., y: bottom + PLAYER_SIZE.1 / 2. * SPRITE_SCALE + 5., z: 10. },
			scale: Vec3::new(SPRITE_SCALE, SPRITE_SCALE, 1.),
			..Default::default()
		},
		..Default::default()
	})
        .insert(Player)
        .insert(SpriteSize::from(PLAYER_SIZE))
        .insert(Velocity {x: 0., y: 0.})
        .insert(Movable { auto_despawn: false });
}

fn player_keyboard_event_system(kb: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity, With<Player>>) {
    if let Ok(mut velocity) = query.get_single_mut() {
        velocity.x = if kb.pressed(KeyCode::ArrowLeft) {
            -1.
        } else if kb.pressed(KeyCode::ArrowRight) {
            1.
        } else {
            0.
        };
    }    
}
