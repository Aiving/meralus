use meralus_world::{Property, PropertyValue};

use crate::Block;

pub struct AirBlock;

impl Block for AirBlock {
    fn get_properties(&self) -> Vec<Property> {
        Vec::new()
    }
}

pub struct DirtBlock;

impl Block for DirtBlock {
    fn get_properties(&self) -> Vec<Property> {
        Vec::new()
    }
}

pub struct GrassBlock {
    is_snowy: bool,
}

impl Block for GrassBlock {
    fn get_properties(&self) -> Vec<Property> {
        vec![Property {
            name: "snowy",
            value: PropertyValue::Boolean(self.is_snowy),
        }]
    }
}
