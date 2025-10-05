use crate::color::Color;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;
use image::{DynamicImage, GenericImageView};

static IMAGE_REG: Lazy<RwLock<HashMap<u32, DynamicImage>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Copy)]
pub enum Axis { U, V }

#[derive(Debug, Clone, Copy)]
pub enum Texture {
    Checker { color1: Color, color2: Color, scale: f32 },
    Stripes { color1: Color, color2: Color, scale: f32, axis: Axis },
    MarbleProc { color1: Color, color2: Color, scale: f32 },
    Image { id: u32, scale: f32 },
}

pub fn register_image(id: u32, path: &str) -> bool {
    match image::open(path) {
        Ok(img) => {
            if let Ok(mut map) = IMAGE_REG.write() { map.insert(id, img); return true; }
            false
        },
        Err(_) => false,
    }
}

impl Texture {
    pub fn sample(&self, u: f32, v: f32) -> Color {
        match *self {
            Texture::Checker { color1, color2, scale } => {
                let s = (u * scale).floor() as i32 + (v * scale).floor() as i32;
                if s % 2 == 0 { color1 } else { color2 }
            }
            Texture::Stripes { color1, color2, scale, axis } => {
                let t = match axis { Axis::U => u, Axis::V => v };
                if ((t * scale).floor() as i32) % 2 == 0 { color1 } else { color2 }
            }
            Texture::MarbleProc { color1, color2, scale } => {
                // Patrón simple de mármol usando senoides combinadas
                let s = ((u * scale).sin() + (v * scale * 1.5).sin()) * 0.5;
                let t = 0.5 * (s + 1.0);
                color1.blend(color2, t)
            }
            Texture::Image { id, scale } => {
                if let Ok(map) = IMAGE_REG.read() {
                    if let Some(img) = map.get(&id) {
                        let (w, h) = img.dimensions();
                        let uu = (u * scale).fract();
                        let vv = (v * scale).fract();
                        let x = ((uu * w as f32) as u32).min(w - 1);
                        let y = ((vv * h as f32) as u32).min(h - 1);
                        let px = img.get_pixel(x, y);
                        return Color::new(px[0] as f32, px[1] as f32, px[2] as f32);
                    }
                }
                // Fallback si no está registrada la imagen
                Color::new(200.0, 200.0, 200.0)
            }
        }
    }
}
