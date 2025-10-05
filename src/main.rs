use nalgebra_glm::{Vec3, normalize};
use minifb::{Key, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;

mod framebuffer;
mod ray_intersect;
mod cube;
mod sphere;
mod color;
mod camera;
mod light;
mod material;
mod skybox;
mod texture;
mod ray_casting;

use framebuffer::Framebuffer;
use cube::Cube;
use sphere::Sphere;
use color::Color;
use ray_intersect::{Intersect, RayIntersect};
use camera::Camera;
use light::Light;
use material::Material;
use skybox::Skybox;
use texture::{Texture, register_image};
use crate::ray_casting as fast;

const SHADOW_BIAS: f32 = 1e-4;
const MAX_RAY_DEPTH: u32 = 3;

fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

fn refract(incident: &Vec3, normal: &Vec3, eta: f32) -> Option<Vec3> {
    let cosi = (-incident.dot(normal)).clamp(-1.0, 1.0);
    let mut n = *normal;
    let mut etai = 1.0;
    let mut etat = eta;
    let mut cosi_local = cosi;
    if cosi < 0.0 { // inside the object
        cosi_local = -cosi;
        n = -n;
        std::mem::swap(&mut etai, &mut etat);
    }
    let eta_ratio = etai / etat;
    let k = 1.0 - eta_ratio * eta_ratio * (1.0 - cosi_local * cosi_local);
    if k < 0.0 { None } else {
        Some(eta_ratio * *incident + (eta_ratio * cosi_local - k.sqrt()) * n)
    }
}

fn cast_shadow(intersect: &Intersect, light: &Light, objects: &[Box<dyn RayIntersect>]) -> f32 {
    let light_dir = (light.position - intersect.point).normalize();
    let light_distance = (light.position - intersect.point).magnitude();

    let offset_normal = intersect.normal * SHADOW_BIAS;
    let shadow_origin = if light_dir.dot(&intersect.normal) < 0.0 {
        intersect.point - offset_normal
    } else {
        intersect.point + offset_normal
    };

    for object in objects {
        let shadow_i = object.ray_intersect(&shadow_origin, &light_dir);
        if shadow_i.is_intersecting && shadow_i.distance < light_distance {
            return 0.3; // Sombra parcial
        }
    }
    1.0
}

fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3,
             objects: &[Box<dyn RayIntersect>],
             lights: &[Light],
             depth: u32) -> Color {
    if depth > MAX_RAY_DEPTH {
        return Skybox::sample_color(ray_direction);
    }

    let mut closest = Intersect::empty();
    let mut z = f32::INFINITY;

    for obj in objects {
        let i = obj.ray_intersect(ray_origin, ray_direction);
        if i.is_intersecting && i.distance < z {
            z = i.distance;
            closest = i;
        }
    }

    if !closest.is_intersecting {
        return Skybox::sample_color(ray_direction);
    }

    // base difusa: textura si existe y hay UV
    let mut base_diffuse = closest.material.diffuse;
    if let (Some(tex), Some((u, v))) = (closest.material.texture, closest.uv) {
        base_diffuse = tex.sample(u.fract(), v.fract());
    }

    // Luz ambiental + directa
    let mut local = base_diffuse * 0.1; // luz ambiental

    for light in lights {
        let light_dir = (light.position - closest.point).normalize();
        let intensity = cast_shadow(&closest, light, objects);

        let diffuse_strength = closest.normal.dot(&light_dir).max(0.0);
        let diffuse = base_diffuse * diffuse_strength * intensity;

        let reflect_dir = reflect(&-light_dir, &closest.normal);
        let view_dir = (-ray_direction).normalize();
        let specular = light.color * closest.material.albedo[1]
            * view_dir.dot(&reflect_dir).max(0.0).powf(closest.material.specular) * intensity;

        local = local + diffuse + specular;
    }

    // Reflexión / Refracción / Emisión
    let r = closest.material.reflectivity.clamp(0.0, 1.0);
    let t = closest.material.transparency.clamp(0.0, 1.0);
    let base_w = (1.0 - r - t).max(0.0);

    let mut refl_col = Color::black();
    if r > 0.0 && depth < MAX_RAY_DEPTH {
        let dir = reflect(&ray_direction.normalize(), &closest.normal).normalize();
        let bias = closest.normal * SHADOW_BIAS;
        let origin = if dir.dot(&closest.normal) < 0.0 { closest.point - bias } else { closest.point + bias };
        refl_col = cast_ray(&origin, &dir, objects, lights, depth + 1);
    }

    let mut refr_col = Color::black();
    if t > 0.0 && depth < MAX_RAY_DEPTH {
        let eta = closest.material.ior.max(1.0);
        if let Some(dir) = refract(&ray_direction.normalize(), &closest.normal, eta) {
            let bias = closest.normal * SHADOW_BIAS;
            let origin = if dir.dot(&closest.normal) < 0.0 { closest.point - bias } else { closest.point + bias };
            refr_col = cast_ray(&origin, &dir.normalize(), objects, lights, depth + 1);
        }
    }

    let mut out_color = local * base_w + refl_col * r + refr_col * t;

    if let Some(em) = closest.material.emission { out_color = out_color + em; }

    out_color
}

fn render(framebuffer: &mut Framebuffer, objects: &[Box<dyn RayIntersect>],
          camera: &Camera, lights: &[Light]) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let fov = PI / 3.0;
    let scale = (fov * 0.5).tan();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            let px = (2.0 * (x as f32 + 0.5) / width - 1.0) * aspect_ratio * scale;
            let py = (1.0 - 2.0 * (y as f32 + 0.5) / height) * scale;

            let dir = normalize(&Vec3::new(px, py, -1.0));
            let world_dir = camera.basis_change(&dir);
            let color = cast_ray(&camera.position, &world_dir, objects, lights, 0);

            framebuffer.set_current_color(color.to_hex());
            framebuffer.point(x, y);
        }
    }
}

fn main() {
    let width = 800;
    let height = 600;
    let mut fb = Framebuffer::new(width, height);
    let mut window = Window::new("Museo Raytracing", width, height, WindowOptions::default()).unwrap();

    // --- Materiales base ---
    let marble = Material::new(Color::new(220.0, 220.0, 230.0), 20.0, [0.8, 0.2]);
    let _gold = Material::new(Color::new(255.0, 215.0, 0.0), 80.0, [0.7, 0.3]);
    let _copper = Material::new(Color::new(184.0, 115.0, 51.0), 50.0, [0.7, 0.3]);

    // Registrar textura de mármol para pedestales
    let _ = register_image(1, "src/assets/marmol.jpg");

    // --- Objetos ---
    let mut objects: Vec<Box<dyn RayIntersect>> = Vec::new();

    // Suelo con textura checker
    objects.push(Box::new(Cube {
        center: Vec3::new(0.0, -1.5, 0.0),
        size: 50.0,
        material: Material::new(Color::new(245.0, 245.0, 245.0), 10.0, [0.8, 0.2])
            .with_texture(Texture::Checker { color1: Color::new(240.0, 240.0, 240.0), color2: Color::new(210.0, 210.0, 210.0), scale: 8.0 }),
    }));

    // Disposición circular de pedestales y esferas
    let count = 12;
    let radius_ring = 6.0;

    // Definir 12 materiales de esferas
    let sphere_materials: Vec<Material> = vec![
        // 1. Metal pulido
        Material::new(Color::new(200.0, 200.0, 200.0), 120.0, [0.2, 0.8]).with_reflectivity(0.9),
        // 2. Metal rugoso
        Material::new(Color::new(180.0, 180.0, 180.0), 20.0, [0.6, 0.4]).with_reflectivity(0.6).with_roughness(1.0),
        // 3. Plástico brillante
        Material::new(Color::new(80.0, 120.0, 255.0), 80.0, [0.8, 0.2]).with_reflectivity(0.1),
        // 4. Vidrio transparente
        Material::new(Color::new(200.0, 255.0, 255.0), 100.0, [0.2, 0.8]).with_transparency(0.9).with_ior(1.5).with_reflectivity(0.05),
        // 5. Vidrio esmerilado
        Material::new(Color::new(220.0, 240.0, 240.0), 20.0, [0.2, 0.8]).with_transparency(0.9).with_ior(1.5).with_reflectivity(0.05).with_roughness(1.0),
        // 6. Agua
        Material::new(Color::new(180.0, 200.0, 255.0), 20.0, [0.1, 0.9]).with_transparency(0.98).with_ior(1.33).with_reflectivity(0.02),
        // 7. Mármol (procedural)
        Material::new(Color::new(230.0, 230.0, 240.0), 30.0, [0.8, 0.2]).with_texture(Texture::MarbleProc { color1: Color::new(230.0, 230.0, 240.0), color2: Color::new(180.0, 180.0, 200.0), scale: 12.0 }),
        // 8. Oro
        Material::new(Color::new(255.0, 215.0, 0.0), 80.0, [0.7, 0.3]).with_reflectivity(0.8),
        // 9. Cobre
        Material::new(Color::new(184.0, 115.0, 51.0), 50.0, [0.7, 0.3]).with_reflectivity(0.75),
        // 10. Neón (emisión)
        Material::new(Color::new(30.0, 30.0, 30.0), 10.0, [1.0, 0.0]).with_emission(Color::new(0.0, 255.0, 180.0)),
        // 11. Niebla/volumen (aprox)
        Material::new(Color::new(200.0, 200.0, 220.0), 5.0, [1.0, 0.0]).with_transparency(0.5).with_ior(1.0),
        // 12. Espejo
        Material::black().with_reflectivity(1.0),
    ];

    for i in 0..count {
        let angle = 2.0 * PI * (i as f32) / (count as f32);
        let px = radius_ring * angle.cos();
        let pz = radius_ring * angle.sin();

        // pedestal con textura de imagen marmol
        let pedestal_mat = marble.with_texture(Texture::Image { id: 1, scale: 2.0 });
        objects.push(Box::new(Cube {
            center: Vec3::new(px, -0.5, pz),
            size: 1.0,
            material: pedestal_mat,
        }));

        // esfera encima con material específico
        let sphere_y = 0.8;
        let sphere_mat = sphere_materials[i as usize % sphere_materials.len()];
        objects.push(Box::new(Sphere {
            center: Vec3::new(px, sphere_y, pz),
            radius: 0.6,
            material: sphere_mat,
        }));
    }

    // Luces
    let lights = [
        Light::new(Vec3::new(5.0, 5.0, 5.0), Color::new(255.0, 255.0, 240.0), 1.2),
        Light::new(Vec3::new(-5.0, 4.0, 2.0), Color::new(200.0, 200.0, 255.0), 0.8),
    ];

    // Cámara
    let mut camera = Camera::new(Vec3::new(0.0, 2.0, 12.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let mut yaw = 0.0;
    let mut pitch = 0.0;
    let mut distance = 0.0;

    // --- Loop ---
    while window.is_open() {
        if window.is_key_down(Key::Escape) { break; }

        // Rotación
        if window.is_key_down(Key::A) { yaw += 0.02; }
        if window.is_key_down(Key::D) { yaw -= 0.02; }
        if window.is_key_down(Key::W) { pitch += 0.02; }
        if window.is_key_down(Key::S) { pitch -= 0.02; }

        // Zoom
        if window.is_key_down(Key::Up) { distance -= 0.1; }
        if window.is_key_down(Key::Down) { distance += 0.1; }

        camera.orbit(yaw * 0.02, pitch * 0.02);
        camera.zoom(distance * 0.1);
        yaw *= 0.95;
        pitch *= 0.95;
        distance *= 0.95;

        fb.clear();
        fast::render(&mut fb, &objects, &camera, &lights);
        window.update_with_buffer(&fb.buffer, width, height).unwrap();

        std::thread::sleep(Duration::from_millis(16));
    }
}
