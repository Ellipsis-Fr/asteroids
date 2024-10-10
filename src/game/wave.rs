use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::Resource;
use yaml_rust2::{Yaml, YamlLoader};

use super::meteor::{self, MeteorDefinition};
extern crate yaml_rust2;

static mut WAVE_COUNT: AtomicUsize = AtomicUsize::new(1);
const WAVE_DATA: &str = "../../resources/waves.yml";


#[derive(Debug, Resource)]
pub struct Wave {
    meteors: Vec<MeteorDefinition>,
    enemies: i32
}

impl Wave {
    pub fn new() -> Self {
        let yaml = Self::get_yaml_access();

        let (meteors_wave_one, enemies_wave_one) = Self::parse_wave_data(yaml, Self::get_wave_count());
        Self::increment_wave_count();

        Wave { meteors: meteors_wave_one, enemies: enemies_wave_one }
    }

    fn get_yaml_access() -> Yaml {
        let yaml_file = std::fs::read_to_string(WAVE_DATA).unwrap();
        let yaml_file_content = YamlLoader::load_from_str(&yaml_file).unwrap();
        yaml_file_content[0].clone()
    }


    fn increment_wave_count() {
        unsafe {
            WAVE_COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn get_wave_count() -> usize {
        unsafe {
            WAVE_COUNT.load(Ordering::SeqCst)
        }
    }

    fn parse_wave_data(yaml: Yaml, index: usize) -> (Vec<MeteorDefinition>, i32) {
        let mut meteors_definition = Vec::new();
        
        for meteor in yaml[index]["meteors"].clone() {
            meteors_definition.push(MeteorDefinition {
                weight: meteor["weight"].as_i64().unwrap() as i32,
                speed: [
                    meteor["speed"][0].as_f64().unwrap() as f32,
                    meteor["speed"][1].as_f64().unwrap() as f32,
                ],
                kind: meteor["kind"].as_i64().unwrap() as u8,
                level: meteor["level"].as_i64().unwrap() as u8,
            });
        }

        (meteors_definition, yaml[index]["enemies"].as_i64().unwrap() as i32)
    }

    pub fn has_meteors(&self) -> bool {
        !self.meteors.is_empty()
    }

    pub fn get_meteors(&mut self) -> &mut Vec<MeteorDefinition> {
        &mut self.meteors
    }

    pub fn get_enemies(&mut self) -> &mut i32 {
        &mut self.enemies
    }
}