use nalgebra_glm::Vec3;
use crate::ray_intersect::{RayIntersect, Intersect};
use crate::material::Material;

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let l = self.center - ray_origin;
        let tca = l.dot(ray_direction);
        if tca < 0.0 {
            return Intersect::empty();
        }

        let d2 = l.dot(&l) - tca * tca;
        let radius2 = self.radius * self.radius;
        if d2 > radius2 {
            return Intersect::empty();
        }

        let thc = (radius2 - d2).sqrt();
        let t0 = tca - thc;
        let t1 = tca + thc;

        let t = if t0 < 0.0 { t1 } else { t0 };
        if t < 0.0 {
            return Intersect::empty();
        }

        let point = ray_origin + ray_direction * t;
        let normal = (point - self.center).normalize();

        // UV esféricas
        let dir = (point - self.center).normalize();
        let u = 0.5 + dir.z.atan2(dir.x) / (2.0 * std::f32::consts::PI);
        let v = 0.5 - dir.y.asin() / std::f32::consts::PI;

        Intersect::new(point, normal, t, self.material).with_uv(u, v)
    }
}
