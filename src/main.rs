#![allow(unused)]
mod game;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{plugin::{NoUserData, RapierPhysicsPlugin}, render::RapierDebugRenderPlugin};
use game::GamePlugin;

use std::collections::HashSet;

use bevy::{core::FrameCount, diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, input::gamepad::{self, ButtonSettingsError}, math::Vec3Swizzles, prelude::*, window::{self, PresentMode, PrimaryWindow, WindowTheme}};


// TODO : - Intégration Vaisseau (prise en compte de la forme, définition taille)
// ! Ne sais pas faire précédent point, nécessite de se former sur les shaders et meshs

// TODO : - Gestion du visuel d'accélération

// TODO : - Téléportation Vaisseau (trouver une zone sur la map où aucun danger n'est présent)

// TODO : - Astéroïdes (Intégtation, division, dplct)

// TODO : - Vaisseau Enemie

// TODO : - Scores


fn main() {
    App::new()
		.add_plugins((
			DefaultPlugins.set(WindowPlugin {
				primary_window: Some(Window {
					title: "Rust Asteroids!".into(),
					name: Some("bevy.app".into()),
					resolution: (900., 700.).into(),
					present_mode: PresentMode::AutoVsync,
					// Tells wasm not to override default event handling, like F5, Ctrl+R etc.
					prevent_default_event_handling: false,
					window_theme: Some(WindowTheme::Dark),
					enabled_buttons: bevy::window::EnabledButtons {
						maximize: false,
						..Default::default()
					},
					resizable: false,
					// This will spawn an invisible window
					// The window will be made visible in the make_visible() system after 3 frames.
					// This is useful when you want to avoid the white window that shows up before the GPU is ready to render the app.
					visible: false,
					..default()
				}),
				..default()
			}),
			// LogDiagnosticsPlugin::default(),
			// FrameTimeDiagnosticsPlugin,
		))
		// .add_plugins(RapierDebugRenderPlugin::default())
		.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0))
		// .add_plugins(InspectableRapierPlugin)
		// .add_plugins(WorldInspectorPlugin::default())
		.add_plugins(GamePlugin)
		.run();
}

