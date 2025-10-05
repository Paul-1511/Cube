use nalgebra_glm::Vec3;
use crate::ray_intersect::{RayIntersect, Intersect};
use crate::material::Material;

pub struct Cube {
    pub center: Vec3,
    pub size: f32,
    pub material: Material,
}

impl RayIntersect for Cube {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let half_size = self.size / 2.0;
        let min = self.center - Vec3::new(half_size, half_size, half_size);
        let max = self.center + Vec3::new(half_size, half_size, half_size);

        let inv_dir = Vec3::new(
            1.0 / ray_direction.x,
            1.0 / ray_direction.y,
            1.0 / ray_direction.z,
        );

        let t1 = (min.x - ray_origin.x) * inv_dir.x;
        let t2 = (max.x - ray_origin.x) * inv_dir.x;
        let t3 = (min.y - ray_origin.y) * inv_dir.y;
        let t4 = (max.y - ray_origin.y) * inv_dir.y;
        let t5 = (min.z - ray_origin.z) * inv_dir.z;
        let t6 = (max.z - ray_origin.z) * inv_dir.z;

        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));

        if tmax < 0.0 || tmin > tmax {
            return Intersect::empty();
        }

        let t = if tmin > 0.0 { tmin } else { tmax };
        if t <= 0.0 {
            return Intersect::empty();
        }

        let point = ray_origin + ray_direction * t;
        let local_point = point - self.center;

        // Determinar la normal basada en la cara más cercana
        let abs_x = local_point.x.abs();
        let abs_y = local_point.y.abs();
        let abs_z = local_point.z.abs();

        let normal = if abs_x > abs_y && abs_x > abs_z {
            Vec3::new(local_point.x.signum(), 0.0, 0.0)
        } else if abs_y > abs_z {
            Vec3::new(0.0, local_point.y.signum(), 0.0)
        } else {
            Vec3::new(0.0, 0.0, local_point.z.signum())
        };

        // UV por cara (proyección planar en el eje perpendicular a la cara)
        let half = half_size;
        let (u, v) = if normal.x.abs() > 0.0 {
            // cara +/-X, usar Z e Y
            (((local_point.z + half) / (2.0 * half)), ((local_point.y + half) / (2.0 * half)))
        } else if normal.y.abs() > 0.0 {
            // cara +/-Y, usar X y Z
            (((local_point.x + half) / (2.0 * half)), ((local_point.z + half) / (2.0 * half)))
        } else {
            // cara +/-Z, usar X e Y
            (((local_point.x + half) / (2.0 * half)), ((local_point.y + half) / (2.0 * half)))
        };

        Intersect::new(point, normal, t, self.material).with_uv(u, v)
    }
}