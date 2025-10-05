use nalgebra_glm::Vec3;
use rayon::prelude::*;

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;

const SHADOW_BIAS: f32 = 1e-4;
const MAX_RAY_DEPTH: u32 = 3;

#[inline(always)]
fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * incident.dot(normal) * normal
}

#[inline(always)]
fn refract(incident: &Vec3, normal: &Vec3, eta: f32) -> Option<Vec3> {
    let cosi = (-incident.dot(normal)).clamp(-1.0, 1.0);
    let mut n = *normal;
    let mut etai = 1.0;
    let mut etat = eta;
    let mut cosi_local = cosi;
    if cosi < 0.0 {
        cosi_local = -cosi;
        n = -n;
        core::mem::swap(&mut etai, &mut etat);
    }
    let eta_ratio = etai / etat;
    let k = 1.0 - eta_ratio * eta_ratio * (1.0 - cosi_local * cosi_local);
    if k < 0.0 { None } else {
        Some(eta_ratio * *incident + (eta_ratio * cosi_local - k.sqrt()) * n)
    }
}

#[inline(always)]
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
            return 0.3; // sombra parcial
        }
    }
    1.0
}

fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Box<dyn RayIntersect>],
    lights: &[Light],
    depth: u32,
) -> Color {
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

    // textura base si existe
    let mut base_diffuse = closest.material.diffuse;
    if let (Some(tex), Some((u, v))) = (closest.material.texture, closest.uv) {
        base_diffuse = tex.sample(u.fract(), v.fract());
    }

    // iluminación local
    let mut local = base_diffuse * 0.1; // ambiental

    for light in lights {
        let light_dir = (light.position - closest.point).normalize();
        let intensity = cast_shadow(&closest, light, objects);

        let diffuse_strength = closest.normal.dot(&light_dir).max(0.0);
        let diffuse = base_diffuse * diffuse_strength * intensity;

        let reflect_dir = reflect(&-light_dir, &closest.normal);
        let view_dir = (-ray_direction).normalize();
        let specular = light.color
            * closest.material.albedo[1]
            * view_dir.dot(&reflect_dir).max(0.0).powf(closest.material.specular)
            * intensity;

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
        let origin = if dir.dot(&closest.normal) < 0.0 {
            closest.point - bias
        } else {
            closest.point + bias
        };
        refl_col = cast_ray(&origin, &dir, objects, lights, depth + 1);
    }

    let mut refr_col = Color::black();
    if t > 0.0 && depth < MAX_RAY_DEPTH {
        let eta = closest.material.ior.max(1.0);
        if let Some(dir) = refract(&ray_direction.normalize(), &closest.normal, eta) {
            let bias = closest.normal * SHADOW_BIAS;
            let origin = if dir.dot(&closest.normal) < 0.0 {
                closest.point - bias
            } else {
                closest.point + bias
            };
            refr_col = cast_ray(&origin, &dir.normalize(), objects, lights, depth + 1);
        }
    }

    let mut out_color = local * base_w + refl_col * r + refr_col * t;
    if let Some(em) = closest.material.emission {
        out_color = out_color + em;
    }

    out_color
}

pub fn render(
    framebuffer: &mut Framebuffer,
    objects: &[Box<dyn RayIntersect>],
    camera: &crate::camera::Camera,
    lights: &[Light],
) {
    let width = framebuffer.width as u32;
    let height = framebuffer.height as u32;
    let fw = width as f32;
    let fh = height as f32;
    let aspect_ratio = fw / fh;
    let fov = std::f32::consts::PI / 3.0;
    let scale = (fov * 0.5).tan();

    // base de cámara
    let forward = (camera.center - camera.position).normalize();
    let right = forward.cross(&camera.up).normalize();
    let up = right.cross(&forward).normalize();

    // precálculo de px por columna
    let mut px_row: Vec<f32> = Vec::with_capacity(width as usize);
    for x in 0..width {
        let px = (2.0 * (x as f32 + 0.5) / fw - 1.0) * aspect_ratio * scale;
        px_row.push(px);
    }

    // render paralelo con Rayon
    let buf_len = (width * height) as usize;

    framebuffer
        .buffer
        .par_iter_mut()
        .enumerate()
        .for_each(|(idx, pixel)| {
            let x = (idx as u32) % width;
            let y = (idx as u32) / width;

            // acceso sin bounds-check
            let px = unsafe { *px_row.get_unchecked(x as usize) };
            let py = (1.0 - 2.0 * (y as f32 + 0.5) / fh) * scale;

            let dir_cam = Vec3::new(px, py, -1.0);
            let world_dir = (dir_cam.x * right + dir_cam.y * up - dir_cam.z * forward).normalize();

            let col = cast_ray(&camera.position, &world_dir, objects, lights, 0);

            *pixel = col.to_hex();
        });
}
