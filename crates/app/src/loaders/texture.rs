use meralus_engine::WindowDisplay;
use meralus_engine::glium::texture::{CompressedTexture2d, RawImage2d};
use owo_colors::OwoColorize;
use std::{collections::HashMap, path::Path};

#[derive(Default)]
pub struct TextureLoader {
    texture_name_map: HashMap<String, usize>,
    textures: Vec<CompressedTexture2d>,
}

impl TextureLoader {
    pub fn get_id<T: AsRef<str>>(&self, name: T) -> Option<usize> {
        self.texture_name_map.get(name.as_ref()).copied()
    }

    pub fn get_by_id(&self, id: usize) -> Option<&CompressedTexture2d> {
        self.textures.get(id)
    }

    pub fn get_by_name<T: AsRef<str>>(&self, name: T) -> Option<&CompressedTexture2d> {
        self.texture_name_map
            .get(name.as_ref())
            .and_then(|&texture_id| self.textures.get(texture_id))
    }

    pub fn load<P: AsRef<Path>>(&mut self, display: &WindowDisplay, path: P) {
        let path = path.as_ref();

        println!(
            "[{}] Loading texture at {}",
            "INFO/TextureLoader".bright_green(),
            path.display().bright_blue().bold()
        );

        if let Some(name) = path.file_stem() {
            let name = name.to_string_lossy();

            match image::ImageReader::open(path).and_then(image::ImageReader::with_guessed_format) {
                Ok(value) => {
                    if let Ok(value) = value.decode() {
                        let image = value.to_rgba8();
                        let dimensions = image.dimensions();

                        let image =
                            RawImage2d::from_raw_rgba_reversed(&image.into_raw(), dimensions);

                        if let Ok(value) = CompressedTexture2d::new(display, image) {
                            let texture_id = self.textures.len();

                            self.textures.push(value);
                            self.texture_name_map.insert(name.to_string(), texture_id);
                        }
                    }
                }
                Err(err) => panic!("Failed to load texture: {err}"),
            }
        }
    }
}
