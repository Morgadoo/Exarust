//! Turns a cloth's draw passes into materials the renderer submits.

use exa_physics::cloth::Cloth;
use exa_render::material::ExaMaterial;

/// One material per pass the cloth submits. A double-sided cloth culls the
/// front and then the back. A single-sided cloth culls only the back.
pub fn cloth_draw_materials(base: &ExaMaterial, cloth: &Cloth) -> Vec<ExaMaterial> {
    base.cloth_draws(&cloth.submitted_culls())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;
    use bevy::render::render_resource::Face;
    use exa_render::material::ExaMaterialKey;

    #[test]
    fn a_mesh_cloth_submits_a_front_pass_and_a_back_pass() {
        let vertices = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let cloth = Cloth::from_mesh(&vertices, &[[0, 1, 3], [1, 2, 3]], &[0]);
        let draws = cloth_draw_materials(&ExaMaterial::default(), &cloth);
        assert_eq!(draws.len(), 2);
        assert_eq!(
            ExaMaterialKey::from(&draws[0]).cloth_cull(),
            Some(Face::Front)
        );
        assert_eq!(
            ExaMaterialKey::from(&draws[1]).cloth_cull(),
            Some(Face::Back)
        );

        let mut single = cloth;
        single.double_sided = false;
        let draws = cloth_draw_materials(&ExaMaterial::default(), &single);
        assert_eq!(draws.len(), 1);
        assert_eq!(
            ExaMaterialKey::from(&draws[0]).cloth_cull(),
            Some(Face::Back)
        );
    }
}
