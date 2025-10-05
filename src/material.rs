use crate::color::Color;
use crate::texture::Texture;

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub diffuse: Color,
    pub specular: f32,
    pub albedo: [f32; 2],
    pub is_crystal: bool,
    pub texture: Option<Texture>,
    pub reflectivity: f32,
    pub transparency: f32,
    pub ior: f32,
    pub roughness: f32,
    pub emission: Option<Color>,
}

impl Material {
    pub fn new(diffuse: Color, specular: f32, albedo: [f32; 2]) -> Self {
        Self {
            diffuse,
            specular,
            albedo,
            is_crystal: false,
            texture: None,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.0,
            roughness: 0.0,
            emission: None,
        }
    }

    pub fn crystal(diffuse: Color, specular: f32, albedo: [f32; 2]) -> Self {
        Self {
            diffuse,
            specular,
            albedo,
            is_crystal: true,
            texture: None,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.5,
            roughness: 0.0,
            emission: None,
        }
    }

    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_reflectivity(mut self, r: f32) -> Self { self.reflectivity = r; self }
    pub fn with_transparency(mut self, t: f32) -> Self { self.transparency = t; self }
    pub fn with_ior(mut self, ior: f32) -> Self { self.ior = ior; self }
    pub fn with_roughness(mut self, r: f32) -> Self { self.roughness = r; self }
    pub fn with_emission(mut self, c: Color) -> Self { self.emission = Some(c); self }

    pub fn black() -> Self {
        Self {
            diffuse: Color::new(0.0, 0.0, 0.0),
            specular: 0.0,
            albedo: [0.0, 0.0],
            is_crystal: false,
            texture: None,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.0,
            roughness: 0.0,
            emission: None,
        }
    }
}
