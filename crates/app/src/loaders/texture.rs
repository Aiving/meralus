use std::{borrow::Borrow, collections::HashMap, hash::Hash, path::Path};

use glam::{UVec2, Vec2, uvec2};
use image::RgbaImage;
use meralus_engine::{
    WindowDisplay,
    glium::{
        Rect, Texture2d,
        texture::{MipmapsOption, RawImage2d},
    },
};
use owo_colors::OwoColorize;

const fn alpha_blend(mut one: u32, mut two: u32) -> (u8, u8, u8, u8) {
    let mut i = (one as i32 & -16777216) as u32 >> 24 & 255;
    let mut j = (two as i32 & -16777216) as u32 >> 24 & 255;
    let mut k = u32::midpoint(i, j);

    if i == 0 && j == 0 {
        i = 1;
        j = 1;
    } else {
        if i == 0 {
            one = two;
            k /= 2;
        }

        if j == 0 {
            two = one;
            k /= 2;
        }
    }

    let l = (one >> 16 & 255) * i;
    let i1 = (one >> 8 & 255) * i;
    let j1 = (one & 255) * i;
    let k1 = (two >> 16 & 255) * j;
    let l1 = (two >> 8 & 255) * j;
    let i2 = (two & 255) * j;
    let j2 = (l + k1) / (i + j);
    let k2 = (i1 + l1) / (i + j);
    let l2 = (j1 + i2) / (i + j);

    (j2 as u8, k2 as u8, l2 as u8, k as u8)
}

const fn blend_colors(a: u32, b: u32, c: u32, d: u32) -> (u8, u8, u8, u8) {
    alpha_blend(pack_rgba(alpha_blend(a, b)), pack_rgba(alpha_blend(c, d)))
}

const fn pack_rgba((r, g, b, a): (u8, u8, u8, u8)) -> u32 {
    (a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32
}

pub struct TextureAtlas<K: Hash + Eq> {
    texture_map: HashMap<K, Rect>,
    next_texture_offset: UVec2,
    atlas: Texture2d,
}

impl<K: Hash + Eq> TextureAtlas<K> {
    pub fn new(display: &WindowDisplay, size: u32) -> Self {
        Self {
            texture_map: HashMap::new(),
            next_texture_offset: UVec2::ZERO,
            atlas: Texture2d::empty(display, size, size).expect("failed to create atlas"),
        }
    }

    pub fn with_mipmaps(display: &WindowDisplay, size: u32, mipmaps: u32) -> Self {
        Self {
            texture_map: HashMap::new(),
            next_texture_offset: UVec2::ZERO,
            atlas: Texture2d::empty_with_mipmaps(
                display,
                MipmapsOption::EmptyMipmapsMax(mipmaps),
                size,
                size,
            )
            .expect("failed to create atlas"),
        }
    }

    pub const fn get_texture(&self) -> &Texture2d {
        &self.atlas
    }

    pub fn get_rect<Q: ?Sized + Hash + Eq>(&self, key: &Q) -> Option<(Vec2, Vec2)>
    where
        K: Borrow<Q>,
    {
        self.texture_map.get(key).copied().map(
            |Rect {
                 left,
                 bottom,
                 width,
                 height,
             }| {
                (
                    Vec2::new(
                        left as f32 / self.atlas.width() as f32,
                        bottom as f32 / self.atlas.height() as f32,
                    ),
                    Vec2::new(
                        width as f32 / self.atlas.width() as f32,
                        height as f32 / self.atlas.height() as f32,
                    ),
                )
            },
        )
    }

    pub fn rects(&self) -> usize {
        self.texture_map.len()
    }

    pub fn generate_mipmaps(&self, level: usize) {
        let buffer = self.atlas.read_to_pixel_buffer();
        let mut levels = vec![Vec::new(); level + 1];

        levels[0] = buffer.read().unwrap();

        for i in 1..=level {
            let pixels = &levels[i - 1];
            let mut data = vec![(0, 0, 0, 0); pixels.len() >> 2];
            let j = self.atlas.width() as usize >> i;
            let k = data.len() / j;
            let l = j << 1;

            for i1 in 0..j {
                for j1 in 0..k {
                    let k1 = 2 * (i1 + j1 * l);
                    let color = blend_colors(
                        pack_rgba(pixels[k1]),
                        pack_rgba(pixels[k1 + 1]),
                        pack_rgba(pixels[k1 + l]),
                        pack_rgba(pixels[k1 + 1 + l]),
                    );

                    data[i1 + j1 * j] = color;
                }
            }

            levels[i] = data;
        }

        for (index, data) in levels.into_iter().enumerate() {
            if index as u32 >= self.atlas.get_mipmap_levels() {
                break;
            }

            let level = self.atlas.mipmap(index as u32).unwrap();
            let [width, height] = [level.width(), level.height()];

            let image = RawImage2d::from_raw_rgba(
                {
                    let mut v = Vec::with_capacity(data.len() * 4);

                    for (a, b, c, d) in data {
                        v.push(a);
                        v.push(b);
                        v.push(c);
                        v.push(d);
                    }

                    v
                },
                (width, height),
            );

            level.write(
                Rect {
                    left: 0,
                    bottom: 0,
                    width,
                    height,
                },
                image,
            );
        }
    }

    pub fn contains<Q: ?Sized + Hash + Eq>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
    {
        self.texture_map.contains_key(key)
    }

    pub fn append(&mut self, key: K, image: RgbaImage) -> (Vec2, Vec2) {
        if let Some(rect) = self.get_rect(&key) {
            return rect;
        }

        let dimensions = image.dimensions();

        let image = RawImage2d::from_raw_rgba_reversed(&image.into_raw(), dimensions);

        let offset = Rect {
            left: self.next_texture_offset.x,
            bottom: self.next_texture_offset.y,
            width: image.width,
            height: image.height,
        };

        self.atlas.write(offset, image);

        self.texture_map.insert(key, offset);

        self.next_texture_offset = uvec2(offset.left + offset.width, offset.bottom);

        (
            Vec2::new(
                offset.left as f32 / self.atlas.width() as f32,
                offset.bottom as f32 / self.atlas.height() as f32,
            ),
            Vec2::new(
                offset.width as f32 / self.atlas.width() as f32,
                offset.height as f32 / self.atlas.height() as f32,
            ),
        )
    }
}

pub struct TextureLoader {
    atlas: TextureAtlas<String>,
}

impl TextureLoader {
    pub const ATLAS_SIZE: u32 = 4096;

    pub fn new(display: &WindowDisplay) -> Self {
        Self {
            atlas: TextureAtlas::with_mipmaps(display, Self::ATLAS_SIZE, 4),
        }
    }

    pub fn get_texture<T: AsRef<str>>(&self, name: T) -> Option<(Vec2, Vec2)> {
        self.atlas.get_rect(name.as_ref())
    }

    pub const fn get_atlas(&self) -> &Texture2d {
        self.atlas.get_texture()
    }

    pub fn get_texture_count(&self) -> usize {
        self.atlas.rects()
    }

    pub fn generate_mipmaps(&mut self, level: usize) {
        self.atlas.generate_mipmaps(level);
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();

        println!(
            "[{}] Loading texture at {}",
            "INFO/TextureLoader".bright_green(),
            path.display().bright_blue().bold()
        );

        if let Some(name) = path.file_stem() {
            let name = name.to_string_lossy();
            let name = name.to_string();

            if self.atlas.contains(&name) {
                return;
            }

            match image::ImageReader::open(path).and_then(image::ImageReader::with_guessed_format) {
                Ok(value) => {
                    if let Ok(value) = value.decode() {
                        let image = value.to_rgba8();

                        self.atlas.append(name, image);
                    }
                }
                Err(err) => panic!("Failed to load texture: {err}"),
            }
        }
    }
}
