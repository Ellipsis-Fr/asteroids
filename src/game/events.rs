use bevy::{ecs::event::Event, math::Vec3};

use super::meteor::MeteorDefinition;


#[derive(Event)]
pub struct MeteorDestructionEvent(pub (MeteorDefinition, Vec3));