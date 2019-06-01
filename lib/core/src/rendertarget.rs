use crate::*;

#[derive(Debug)]
pub enum Error {
    Texture(crate::texture::Error),
    IO(std::io::Error),
    FailedToCreateFramebuffer {message: String}
}

impl From<crate::texture::Error> for Error {
    fn from(other: crate::texture::Error) -> Self {
        Error::Texture(other)
    }
}

impl From<std::io::Error> for Error {
    fn from(other: std::io::Error) -> Self {
        Error::IO(other)
    }
}

pub trait Rendertarget {
    fn bind(&self);
    fn clear(&self);
    fn bind_for_read(&self);
}

// SCREEN RENDER TARGET
pub struct ScreenRendertarget {
    gl: Gl,
    pub width: usize,
    pub height: usize,
    clear_color: Vec4
}

impl ScreenRendertarget
{
    pub fn new(gl: &Gl, width: usize, height: usize, clear_color: Vec4) -> Result<ScreenRendertarget, Error>
    {
        Ok(ScreenRendertarget { gl: gl.clone(), width, height, clear_color })
    }

    #[cfg(target_arch = "x86_64")]
    pub fn pixels(&self, dst_data: &mut [u8])
    {
        self.bind();
        self.gl.read_pixels(0, 0, self.width as u32, self.height as u32, gl::consts::RGB, gl::consts::UNSIGNED_BYTE, dst_data);
    }

    #[cfg(target_arch = "x86_64")]
    pub fn depths(&self, dst_data: &mut [f32])
    {
        self.bind();
        self.gl.read_depths(0, 0, self.width as u32, self.height as u32, gl::consts::DEPTH_COMPONENT, gl::consts::FLOAT, dst_data);
    }
}

impl Rendertarget for ScreenRendertarget
{
    fn bind(&self)
    {
        self.gl.bind_framebuffer(gl::consts::DRAW_FRAMEBUFFER, None);
        self.gl.viewport(0, 0, self.width as i32, self.height as i32);
    }

    fn bind_for_read(&self)
    {
        self.gl.bind_framebuffer(gl::consts::READ_FRAMEBUFFER, None);
    }

    fn clear(&self)
    {
        depth_write(&self.gl,true);
        self.gl.clear_color(self.clear_color.x, self.clear_color.y, self.clear_color.z, self.clear_color.w);
        self.gl.clear(gl::consts::COLOR_BUFFER_BIT | gl::consts::DEPTH_BUFFER_BIT);
    }
}

// COLOR RENDER TARGET
pub struct ColorRendertarget {
    gl: Gl,
    id: gl::Framebuffer,
    pub width: usize,
    pub height: usize,
    pub targets: Vec<Texture2D>,
    pub depth_target: Texture2D,
    pub clear_color: Vec4
}

impl ColorRendertarget
{
    pub fn new(gl: &Gl, width: usize, height: usize, no_targets: usize, clear_color: Vec4) -> Result<ColorRendertarget, Error>
    {
        let id = generate(gl)?;
        bind(gl, &id, width, height);

        let mut draw_buffers = Vec::new();
        let mut targets = Vec::new();
        for i in 0..no_targets {
            draw_buffers.push(gl::consts::COLOR_ATTACHMENT0 + i as u32);
            targets.push(Texture2D::new_as_color_target(gl, width, height, i as u32)?)
        }

        gl.draw_buffers(&draw_buffers);

        let depth_target = Texture2D::new_as_depth_target(gl, width, height)?;
        gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;
        Ok(ColorRendertarget { gl: gl.clone(), id, width, height, targets, depth_target, clear_color })
    }

    #[cfg(target_arch = "x86_64")]
    pub fn pixels(&self, dst_data: &mut [u8])
    {
        self.bind();
        self.gl.read_pixels(0, 0, self.width as u32, self.height as u32, gl::consts::RGB, gl::consts::UNSIGNED_BYTE, dst_data);
    }

    #[cfg(target_arch = "x86_64")]
    pub fn depths(&self, dst_data: &mut [f32])
    {
        self.bind();
        self.gl.read_depths(0, 0, self.width as u32, self.height as u32, gl::consts::DEPTH_COMPONENT, gl::consts::FLOAT, dst_data);
    }
}

impl Rendertarget for ColorRendertarget
{
    fn bind(&self)
    {
        bind(&self.gl, &self.id, self.width, self.height);
    }

    fn bind_for_read(&self)
    {
        self.gl.bind_framebuffer(gl::consts::READ_FRAMEBUFFER, Some(&self.id));
    }

    fn clear(&self)
    {
        depth_write(&self.gl,true);
        self.gl.clear_color(self.clear_color.x, self.clear_color.y, self.clear_color.z, self.clear_color.w);
        self.gl.clear(gl::consts::COLOR_BUFFER_BIT | gl::consts::DEPTH_BUFFER_BIT);
    }
}

impl Drop for ColorRendertarget {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(Some(&self.id));
    }
}

// DEPTH RENDER TARGET
pub struct DepthRenderTarget {
    gl: Gl,
    id: gl::Framebuffer,
    pub width: usize,
    pub height: usize,
    pub target: Texture2D
}

impl DepthRenderTarget
{
    pub fn new(gl: &Gl, width: usize, height: usize) -> Result<DepthRenderTarget, Error>
    {
        let id = generate(gl)?;
        bind(gl, &id, width, height);

        let target = Texture2D::new_as_depth_target(gl, width, height)?;
        gl.check_framebuffer_status().or_else(|message| Err(Error::FailedToCreateFramebuffer {message}))?;
        Ok(DepthRenderTarget { gl: gl.clone(), id, width, height, target })
    }

    #[cfg(target_arch = "x86_64")]
    pub fn depths(&self, dst_data: &mut [f32])
    {
        self.bind();
        self.gl.read_depths(0, 0, self.width as u32, self.height as u32, gl::consts::DEPTH_COMPONENT, gl::consts::FLOAT, dst_data);
    }
}

impl Rendertarget for DepthRenderTarget
{
    fn bind(&self)
    {
        bind(&self.gl, &self.id, self.width, self.height);
    }

    fn bind_for_read(&self)
    {
        self.gl.bind_framebuffer(gl::consts::READ_FRAMEBUFFER, Some(&self.id));
    }

    fn clear(&self)
    {
        depth_write(&self.gl,true);
        self.gl.clear(gl::consts::DEPTH_BUFFER_BIT);
    }
}

impl Drop for DepthRenderTarget {
    fn drop(&mut self) {
        self.gl.delete_framebuffer(Some(&self.id));
    }
}


// COMMON FUNCTIONS
fn generate(gl: &Gl) -> Result<gl::Framebuffer, Error>
{
    gl.create_framebuffer().ok_or_else(|| Error::FailedToCreateFramebuffer {message: "Failed to create framebuffer".to_string()} )
}

fn bind(gl: &Gl, id: &gl::Framebuffer, width: usize, height: usize)
{
    gl.bind_framebuffer(gl::consts::DRAW_FRAMEBUFFER, Some(&id));
    gl.viewport(0, 0, width as i32, height as i32);
}

#[cfg(target_arch = "x86_64")]
pub fn save_screenshot(path: &str, rendertarget: &ScreenRendertarget) -> Result<(), Error>
{
    let mut pixels = vec![0u8; rendertarget.width * rendertarget.height * 3];
    rendertarget.pixels(&mut pixels);
    image::save_buffer(&std::path::Path::new(path), &pixels, rendertarget.width as u32, rendertarget.height as u32, image::RGB(8))?;
    Ok(())
}