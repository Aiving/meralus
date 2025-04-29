mod face;

use std::{collections::HashMap, path::PathBuf};

pub use face::{Axis, Corner, Face};
use glam::Vec2;
use serde::{
    Deserialize, Serialize,
    de::{Error, Visitor},
};

#[derive(Debug, Default, Serialize)]
pub struct BlockFace {
    pub texture: String,
    pub uv: Option<Vec2>,
    pub tint: bool,
    pub cull_face: Option<Face>,
}

impl<'de> Deserialize<'de> for BlockFace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BlockFaceVisitor;

        impl<'de> Visitor<'de> for BlockFaceVisitor {
            type Value = BlockFace;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("expected valid block face")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(BlockFace {
                    texture: v.to_string(),
                    ..Default::default()
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut value = BlockFace::default();
                let mut texture = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "texture" => {
                            texture.replace(map.next_value()?);
                        }
                        "tint" => value.tint = map.next_value()?,
                        "uv" => value.uv = Some(map.next_value()?),
                        "cull_face" => value.cull_face = Some(map.next_value()?),
                        field => Err(Error::unknown_field(field, &["texture", "uv", "cull_face"]))?,
                    }
                }

                value.texture = texture.ok_or_else(|| Error::missing_field("texture"))?;

                Ok(value)
            }
        }

        deserializer.deserialize_any(BlockFaceVisitor)
    }
}

#[derive(Debug)]
pub struct TextureId(pub String, pub PathBuf);

impl<'de> Deserialize<'de> for TextureId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct TextureIdVisitor;

        impl Visitor<'_> for TextureIdVisitor {
            type Value = TextureId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("valid texture id")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let (mod_name, path) = v
                    .split_once(':')
                    .ok_or_else(|| Error::custom("invalid texture id format"))?;

                Ok(TextureId(mod_name.to_string(), PathBuf::from(path)))
            }
        }

        deserializer.deserialize_str(TextureIdVisitor)
    }
}

impl Serialize for TextureId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let id = format!("{}:{}", self.0, self.1.display());

        serializer.serialize_str(&id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Block {
    #[serde(default)]
    pub textures: HashMap<String, TextureId>,
    #[serde(default = "default_ao")]
    pub ambient_occlusion: bool,
    #[serde(default)]
    pub elements: Vec<BlockElement>,
}

const fn default_ao() -> bool {
    true
}

impl Block {
    pub fn from_slice(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }

    pub fn is_transparent(&self) -> bool {
        self.textures.is_empty() && self.elements.is_empty()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockElement {
    #[serde(flatten)]
    pub faces: Faces,
    #[serde(default)]
    pub rotation: i16,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Faces {
    #[serde(rename = "all")]
    All(BlockFace),
    #[serde(rename = "faces")]
    Unique(HashMap<Face, BlockFace>),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::block::Block;

    #[test]
    fn test_block() {
        let data =
            fs::read("/home/aiving/dev/meralus/crates/app/resources/models/air.json").unwrap();

        println!("{:#?}", serde_json::from_slice::<Block>(&data));
    }
}
