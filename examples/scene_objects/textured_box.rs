extern crate image;

use dust::core::program;
use gl;
use dust::traits;
use gust;
use std::rc::Rc;
use dust::core::texture;
use dust::core::texture::Texture;
use dust::core::surface;
use glm;
use dust::camera;
use dust::core::state;
use self::image::{GenericImage};

pub struct TexturedBox {
    program: program::Program,
    model: surface::TriangleSurface,
    texture: texture::Texture2D
}

impl traits::Reflecting for TexturedBox
{
    fn reflect(&self, transformation: &glm::Mat4, camera: &camera::Camera) -> Result<(), traits::Error>
    {
        self.program.cull(state::CullType::BACK);
        self.texture.bind(0);
        self.program.add_uniform_int("texture0", &0)?;
        self.program.add_uniform_mat4("modelMatrix", &transformation)?;
        self.program.add_uniform_mat4("viewMatrix", &camera.get_view())?;
        self.program.add_uniform_mat4("projectionMatrix", &camera.get_projection())?;
        self.program.add_uniform_mat4("normalMatrix", &glm::transpose(&glm::inverse(transformation)))?;

        self.model.render()?;
        Ok(())
    }

}

impl TexturedBox
{
    pub fn create(gl: &gl::Gl) -> Result<Rc<traits::Reflecting>, traits::Error>
    {
        let mesh = gust::models::create_cube().unwrap();
        let program = program::Program::from_resource(gl, "examples/assets/shaders/texture")?;
        let model = surface::TriangleSurface::create(gl, &mesh, &program)?;

        let img = image::open("examples/assets/textures/test_texture.jpg").unwrap();
        let mut texture = texture::Texture2D::create(gl)?;

        texture.fill_with_u8(img.dimensions().0 as usize, img.dimensions().1 as usize, &img.raw_pixels());

        Ok(Rc::new(TexturedBox { program, model, texture }))
    }
}
