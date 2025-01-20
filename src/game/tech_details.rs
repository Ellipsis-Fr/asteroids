use crate::game::WinSize;
use bevy_rapier2d::prelude::Collider;
use bevy::{color::palettes::css::*, prelude::*, window::PrimaryWindow};
use crate::Plugin;
use bevy::prelude::Gizmos;

pub struct TechDetailsPlugin;

impl Plugin for TechDetailsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_grid_system, show_coordinates_system, draw_collider_center));
    }
}

fn draw_collider_center(mut gizmos: Gizmos, collider_query: Query<(&Transform, &Collider)>) {
    for (transform, collider) in collider_query.iter() {
        let center = transform.translation.truncate();
		// dbg!(center);
		let radian_angle_in_quat = transform.rotation.z;
		let sign = if transform.rotation.w > 0.0 { 1. } else { -1. }; 
		let radian_angle = radian_angle_in_quat.asin() * 2. * sign;
		// dbg!((transform.rotation, radian_angle_in_quat, radian_angle));
		
		gizmos.circle_2d(center, 5.0, Color::WHITE);


		
		if let Some(cuboid) = collider.as_cuboid() {
			let half_extents_top_left_edges = cuboid.half_extents();
			let half_extents_bottom_left_edges = half_extents_top_left_edges * Vec2::new(1., -1.);
			
			gizmos.line_2d(center, center + half_extents_top_left_edges, RED);
			gizmos.line_2d(center, center + half_extents_bottom_left_edges, RED);
	
			if radian_angle != 0.0 {
				// dbg!((radian_angle, data_save.0, transform, global_transform));

				let hyp = half_extents_top_left_edges.length();
				let theta_top_left_edges = half_extents_top_left_edges.y.atan2(half_extents_top_left_edges.x);
				let theta_bottom_left_edges = half_extents_bottom_left_edges.y.atan2(half_extents_bottom_left_edges.x);
	
				let new_theta_1 = theta_top_left_edges + radian_angle;
				let new_x_1 = hyp * new_theta_1.cos();
				let new_y_1 = (hyp * new_theta_1.sin());
				gizmos.line_2d(center, center + Vec2::new(new_x_1, new_y_1), BLUE);
	
				let new_theta_2 = theta_bottom_left_edges + radian_angle;
				let new_x_2 = hyp * new_theta_2.cos();
				let new_y_2 = (hyp * new_theta_2.sin());
	
				gizmos.line_2d(center, center + Vec2::new(new_x_2, new_y_2), GREEN);
			}
		} else if let Some(triangle) = collider.as_triangle() {
			// let center = triangle.center().extend(0.) + transform.translation;
			// gizmos.circle_2d(center.truncate(), 5.0, ORANGE);

			let [a, b, c] = triangle.vertices();
			gizmos.line_2d(center, center + a, RED);
			gizmos.line_2d(center, center + b, RED);
			gizmos.line_2d(center, center + c, RED);

			
			if radian_angle != 0.0 {
				let point_rotated_around = | point_to_rotate: Vec3, center: Vec3, rotation: Quat | -> Vec2 {
					let mut transform= Transform::from_translation(point_to_rotate);
					transform.rotate_around(center, rotation);
					transform.translation.truncate()
				};

				let a_rotated = point_rotated_around((center + a).extend(0.), center.clone().extend(0.), transform.rotation.clone());
				let b_rotated = point_rotated_around((center + b).extend(0.), center.clone().extend(0.), transform.rotation.clone());
				let c_rotated = point_rotated_around((center + c).extend(0.), center.clone().extend(0.), transform.rotation.clone());

				gizmos.line_2d(center, a_rotated, BLUE);
				gizmos.line_2d(center, b_rotated, BLUE);
				gizmos.line_2d(center, c_rotated, BLUE);
			}
			
		}
    }
}

fn draw_grid_system(mut gizmos: Gizmos, win_size: Res<WinSize>) {
	let min_x = win_size.x_axys_limit.0;
	let max_x = win_size.x_axys_limit.1;
	
	let min_y = win_size.y_axys_limit.0;
	let max_y = win_size.y_axys_limit.1;

	gizmos.grid_2d(
        Vec2::ZERO,
        0.0,
        UVec2::new(win_size.width as u32 / 50, win_size.height as u32 / 50),
        Vec2::splat(50.),
        Color::linear_rgba(0.5,0.5, 0.5, 0.25)
	);
	
	gizmos.line_2d(Vec2::new(min_x, 0.), Vec2::new(max_x, 0.), WHITE);
	gizmos.line_2d(Vec2::new(0., min_y), Vec2::new(0., max_y), WHITE);
}

fn show_coordinates_system(
	mut commands: Commands,
	windows:  Query<&Window, With<PrimaryWindow>>,
	win_size: Res<WinSize>,
	mouse: Res<ButtonInput<MouseButton>>,
	asset_server: Res<AssetServer>,
	mut query: Query<Entity, With<Text>>
) {
	if mouse.just_pressed(MouseButton::Left) {
		let cursor_position = windows.single().cursor_position().unwrap();
		let (x, y) = (cursor_position.x - win_size.width / 2., win_size.height / 2. - cursor_position.y); 

		let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

		let text_style = TextStyle {
			font,
			font_size: 25.0,
			color: Color::WHITE,
		};
		// Create text
		let text = Text::from_section(format!("x : {x}\ny : {y}"), text_style);
	
		// Create text 2D bundle
		let text_bundle = Text2dBundle {
			text,
			transform: Transform {
				translation: Vec3::new(x - 30., y - 30., 0.0),
				..Default::default()
			},
			..Default::default()
		};
	
		// Spawn text
		commands.spawn(text_bundle);

		

	} else if mouse.just_released(MouseButton::Left) {
		if let Ok(entity) = query.get_single_mut() {
			commands.entity(entity).despawn();
		}
	}
}