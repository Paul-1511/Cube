use nalgebra_glm::Vec3;
use crate::color::Color;
use crate::ray_intersect::{RayIntersect, Intersect};
use once_cell::sync::OnceCell;
use image::DynamicImage;
use std::path::Path;

pub struct Skybox;

static SKYBOX_IMG: OnceCell<Option<DynamicImage>> = OnceCell::new();

fn load_skybox_if_needed() {
    SKYBOX_IMG.get_or_init(|| {
        // Intentar ambas rutas comunes
        let candidates = [
            Path::new("assets/sky.exr"),
            Path::new("src/assets/sky.exr"),
        ];
        for p in &candidates {
            if p.exists() {
                if let Ok(img) = image::open(p) {
                    return Some(img);
                }
            }
        }
        None
    });
}

impl RayIntersect for Skybox {
    fn ray_intersect(&self, _ray_origin: &Vec3, _ray_direction: &Vec3) -> Intersect {
        Intersect::empty()
    }
}

impl Skybox {
    pub fn sample_color(direction: &Vec3) -> Color {
        load_skybox_if_needed();
        if let Some(Some(img)) = SKYBOX_IMG.get() {
            // Convertir a RGB8 para acceso consistente
            let rgb = img.to_rgb8();
            let (w, h) = rgb.dimensions();

            // Mapear dirección -> coords equirectangulares (u,v) desde interior de cúpula
            let dir = direction.normalize();
            // Usamos atan2(x, z) para alinear eje Z al frente, ajustar si fuera necesario
            let u = 0.5 + dir.x.atan2(dir.z) / (2.0 * std::f32::consts::PI);
            let v = 0.5 + dir.y.asin() / std::f32::consts::PI; // mirando desde dentro: invertir segun necesidad

            let x = ((u.fract() * w as f32) as u32).min(w - 1);
            let y = (((1.0 - v.fract()) * h as f32) as u32).min(h - 1); // invertimos V para imagenes equirectangulares tipicas
            let px = rgb.get_pixel(x, y);
            return Color::new(px[0] as f32, px[1] as f32, px[2] as f32);
        }
        // Gradiente de fallback
        let t = 0.5 * (direction.y + 1.0);
        let base = Color::new(135.0, 206.0, 235.0);
        let horizon = Color::new(255.0, 255.0, 255.0);
        horizon.blend(base, t)
    }
}
