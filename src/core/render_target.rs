//!
//! Functionality for rendering to the screen or into textures.
//!

mod clear_state;
#[doc(inline)]
pub use clear_state::*;

mod color_target;
#[doc(inline)]
pub use color_target::*;

mod depth_target;
#[doc(inline)]
pub use depth_target::*;

use crate::core::*;

use crate::context::Framebuffer;
///
/// Adds additional functionality to clear, read from and write to the screen (see [RenderTarget::screen]) or a color texture and
/// a depth texture at the same time (see [RenderTarget::new]).
/// If you only want to perform an operation on either a color texture or depth texture, see [ColorTarget] and [DepthTarget] respectively.
/// A render target purely adds functionality, so it can be created each time it is needed, the actual data is saved in the textures.
///
pub struct RenderTarget<'a> {
    id: Option<Framebuffer>,
    color: Option<ColorTarget<'a>>,
    depth: Option<DepthTarget<'a>>,
    pub(crate) context: Context,
    width: u32,
    height: u32,
}

impl<'a> RenderTarget<'a> {
    ///
    /// Returns the screen render target for this context.
    /// Write to this render target to draw something on the screen.
    ///
    pub fn screen(context: &Context, width: u32, height: u32) -> Self {
        Self {
            context: context.clone(),
            id: None,
            color: None,
            depth: None,
            width,
            height,
        }
    }

    ///
    /// Constructs a new render target that enables rendering into the given [ColorTarget] and [DepthTarget].
    ///
    pub fn new(color: ColorTarget<'a>, depth: DepthTarget<'a>) -> Self {
        let width = color.width();
        let height = color.height();
        Self {
            context: color.context.clone(),
            id: Some(new_framebuffer(&color.context)),
            color: Some(color),
            depth: Some(depth),
            width,
            height,
        }
    }

    ///
    /// Clears the color and depth of this render target as defined by the given clear state.
    ///
    pub fn clear(&self, clear_state: ClearState) -> &Self {
        self.clear_partially(self.scissor_box(), clear_state)
    }

    ///
    /// Clears the color and depth of the part of this render target that is inside the given scissor box.
    ///
    pub fn clear_partially(&self, scissor_box: ScissorBox, clear_state: ClearState) -> &Self {
        self.context.set_scissor(scissor_box);
        self.bind(crate::context::DRAW_FRAMEBUFFER);
        clear_state.apply(&self.context);
        self
    }

    ///
    /// Writes whatever rendered in the `render` closure into this render target.
    ///
    pub fn write(&self, render: impl FnOnce()) -> &Self {
        self.write_partially(self.scissor_box(), render)
    }

    ///
    /// Writes whatever rendered in the `render` closure into the part of this render target defined by the scissor box.
    ///
    pub fn write_partially(&self, scissor_box: ScissorBox, render: impl FnOnce()) -> &Self {
        self.context.set_scissor(scissor_box);
        self.bind(crate::context::DRAW_FRAMEBUFFER);
        render();
        if let Some(ref color) = self.color {
            color.generate_mip_maps();
        }
        self
    }

    ///
    /// Returns the colors of the pixels in this render target.
    /// The number of channels per pixel and the data format for each channel is specified by the generic parameter.
    ///
    /// **Note:** On web, the data format needs to match the data format of the color texture.
    ///
    pub fn read_color<T: TextureDataType>(&self) -> Vec<T> {
        self.read_color_partially(self.scissor_box())
    }

    ///
    /// Returns the colors of the pixels in this render target inside the given scissor box.
    /// The number of channels per pixel and the data format for each channel is specified by the generic parameter.
    ///
    /// **Note:** On web, the data format needs to match the data format of the color texture.
    ///
    pub fn read_color_partially<T: TextureDataType>(&self, scissor_box: ScissorBox) -> Vec<T> {
        if self.id.is_some() && self.color.is_none() {
            panic!("cannot read color from a render target without a color target");
        }
        self.bind(crate::context::DRAW_FRAMEBUFFER);
        self.bind(crate::context::READ_FRAMEBUFFER);
        let mut data_size = std::mem::size_of::<T>();
        // On web, the format needs to be RGBA if the data type is byte.
        if data_size / T::size() as usize == 1 {
            data_size *= 4 / T::size() as usize
        }
        let mut bytes =
            vec![0u8; scissor_box.width as usize * scissor_box.height as usize * data_size];
        unsafe {
            self.context.read_pixels(
                scissor_box.x as i32,
                scissor_box.y as i32,
                scissor_box.width as i32,
                scissor_box.height as i32,
                format_from_data_type::<T>(),
                T::data_type(),
                crate::context::PixelPackData::Slice(&mut bytes),
            );
        }
        let mut pixels = from_byte_slice(&bytes).to_vec();
        flip_y(
            &mut pixels,
            scissor_box.width as usize,
            scissor_box.height as usize,
        );
        pixels
    }

    ///
    /// Returns the depth values in this render target.
    ///
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_depth(&self) -> Vec<f32> {
        self.read_depth_partially(self.scissor_box())
    }

    ///
    /// Returns the depth values in this render target inside the given scissor box.
    ///
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read_depth_partially(&self, scissor_box: ScissorBox) -> Vec<f32> {
        if self.id.is_some() && self.depth.is_none() {
            panic!("cannot read depth from a render target without a depth target");
        }
        self.bind(crate::context::DRAW_FRAMEBUFFER);
        self.bind(crate::context::READ_FRAMEBUFFER);
        let mut pixels = vec![0u8; scissor_box.width as usize * scissor_box.height as usize * 4];
        unsafe {
            self.context.read_pixels(
                scissor_box.x as i32,
                scissor_box.y as i32,
                scissor_box.width as i32,
                scissor_box.height as i32,
                crate::context::DEPTH_COMPONENT,
                crate::context::FLOAT,
                crate::context::PixelPackData::Slice(&mut pixels),
            );
        }
        from_byte_slice(&pixels).to_vec()
    }

    ///
    /// Copies the content of the color and depth texture to the specified scissor box of this render target.
    /// Only copies the channels given by the write mask.
    ///
    pub fn copy_from(
        &self,
        color_texture: Option<&Texture2D>,
        depth_texture: Option<&DepthTargetTexture2D>,
        scissor_box: ScissorBox,
        write_mask: WriteMask,
    ) -> &Self {
        self.write(|| {
            copy_from(
                &self.context,
                color_texture,
                depth_texture,
                scissor_box.into(),
                write_mask,
            )
        })
    }

    ///
    /// Copies the content of the given layers of the color and depth array textures to the specified viewport of this render target.
    /// Only copies the channels given by the write mask.
    ///
    pub fn copy_from_array(
        &self,
        color_texture: Option<(&Texture2DArray, u32)>,
        depth_texture: Option<(&DepthTargetTexture2DArray, u32)>,
        scissor_box: ScissorBox,
        write_mask: WriteMask,
    ) -> &Self {
        self.write(|| {
            copy_from_array(
                &self.context,
                color_texture,
                depth_texture,
                scissor_box.into(),
                write_mask,
            )
        })
    }

    ///
    /// Returns the scissor box that encloses the entire target.
    ///
    pub fn scissor_box(&self) -> ScissorBox {
        ScissorBox::new_at_origo(self.width, self.height)
    }

    ///
    /// Creates a [RenderTarget] with the given low-level [Framebuffer]. Should only be used if the [Framebuffer] is used for something else, ie. to be able
    /// to combine this crate with functionality of another crate. Also see [Self::into_framebuffer].
    ///
    pub fn from_framebuffer(
        context: &Context,
        width: u32,
        height: u32,
        framebuffer: Framebuffer,
    ) -> Self {
        Self {
            id: Some(framebuffer),
            color: None,
            depth: None,
            context: context.clone(),
            width,
            height,
        }
    }

    ///
    /// Transforms this [RenderTarget] into a low-level [Framebuffer]. Should only be used if the [Framebuffer] is used for something else, ie. to be able
    /// to combine this crate with functionality of another crate. Also see [Self::from_framebuffer].
    ///
    pub fn into_framebuffer(mut self) -> Option<Framebuffer> {
        self.id.take()
    }

    fn new_color(color: ColorTarget<'a>) -> Self {
        let width = color.width();
        let height = color.height();
        Self {
            context: color.context.clone(),
            id: Some(new_framebuffer(&color.context)),
            color: Some(color),
            depth: None,
            width,
            height,
        }
    }

    fn new_depth(depth: DepthTarget<'a>) -> Self {
        let width = depth.width();
        let height = depth.height();
        Self {
            context: depth.context.clone(),
            id: Some(new_framebuffer(&depth.context)),
            depth: Some(depth),
            color: None,
            width,
            height,
        }
    }

    fn bind(&self, target: u32) {
        unsafe {
            self.context.bind_framebuffer(target, self.id);
        }
        if let Some(ref color) = self.color {
            color.bind(&self.context);
        }
        if let Some(ref depth) = self.depth {
            depth.bind();
        }
    }
}

impl Drop for RenderTarget<'_> {
    fn drop(&mut self) {
        unsafe {
            if let Some(id) = self.id {
                self.context.delete_framebuffer(id);
            }
        }
    }
}

fn size_with_mip(size: u32, mip: Option<u32>) -> u32 {
    if let Some(mip) = mip {
        size / 2u32.pow(mip)
    } else {
        size
    }
}

fn new_framebuffer(context: &Context) -> crate::context::Framebuffer {
    unsafe {
        context
            .create_framebuffer()
            .expect("Failed creating frame buffer")
    }
}

fn copy_from(
    context: &Context,
    color_texture: Option<&Texture2D>,
    depth_texture: Option<&DepthTargetTexture2D>,
    viewport: Viewport,
    write_mask: WriteMask,
) {
    if color_texture.is_some() || depth_texture.is_some() {
        let fragment_shader_source = if color_texture.is_some() && depth_texture.is_some() {
            "
            uniform sampler2D colorMap;
            uniform sampler2D depthMap;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                color = texture(colorMap, uv);
                gl_FragDepth = texture(depthMap, uv).r;
            }"
        } else if color_texture.is_some() {
            "
            uniform sampler2D colorMap;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                color = texture(colorMap, uv);
            }"
        } else {
            "
            uniform sampler2D depthMap;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                gl_FragDepth = texture(depthMap, uv).r;
            }"
        };
        context
            .effect(fragment_shader_source, |effect| {
                if let Some(tex) = color_texture {
                    effect.use_texture("colorMap", tex);
                }
                if let Some(tex) = depth_texture {
                    effect.use_depth_texture("depthMap", tex);
                }
                effect.apply(
                    RenderStates {
                        depth_test: DepthTest::Always,
                        write_mask: WriteMask {
                            red: color_texture.is_some() && write_mask.red,
                            green: color_texture.is_some() && write_mask.green,
                            blue: color_texture.is_some() && write_mask.blue,
                            alpha: color_texture.is_some() && write_mask.alpha,
                            depth: depth_texture.is_some() && write_mask.depth,
                        },
                        ..Default::default()
                    },
                    viewport,
                )
            })
            .unwrap();
    }
}

fn copy_from_array(
    context: &Context,
    color_texture: Option<(&Texture2DArray, u32)>,
    depth_texture: Option<(&DepthTargetTexture2DArray, u32)>,
    viewport: Viewport,
    write_mask: WriteMask,
) {
    if color_texture.is_some() || depth_texture.is_some() {
        let fragment_shader_source = if color_texture.is_some() && depth_texture.is_some() {
            "
            uniform sampler2DArray colorMap;
            uniform sampler2DArray depthMap;
            uniform int colorLayer;
            uniform int depthLayer;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                color = texture(colorMap, vec3(uv, colorLayer));
                gl_FragDepth = texture(depthMap, vec3(uv, depthLayer)).r;
            }"
        } else if color_texture.is_some() {
            "
            uniform sampler2DArray colorMap;
            uniform int colorLayer;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                color = texture(colorMap, vec3(uv, colorLayer));
            }"
        } else {
            "
            uniform sampler2DArray depthMap;
            uniform int depthLayer;
            in vec2 uv;
            layout (location = 0) out vec4 color;
            void main()
            {
                gl_FragDepth = texture(depthMap, vec3(uv, depthLayer)).r;
            }"
        };
        context
            .effect(fragment_shader_source, |effect| {
                if let Some((tex, layer)) = color_texture {
                    effect.use_texture_array("colorMap", tex);
                    effect.use_uniform("colorLayer", layer as i32);
                }
                if let Some((tex, layer)) = depth_texture {
                    effect.use_depth_texture_array("depthMap", tex);
                    effect.use_uniform("depthLayer", layer as i32);
                }
                effect.apply(
                    RenderStates {
                        depth_test: DepthTest::Always,
                        write_mask: WriteMask {
                            red: color_texture.is_some() && write_mask.red,
                            green: color_texture.is_some() && write_mask.green,
                            blue: color_texture.is_some() && write_mask.blue,
                            alpha: color_texture.is_some() && write_mask.alpha,
                            depth: depth_texture.is_some() && write_mask.depth,
                        },
                        ..Default::default()
                    },
                    viewport,
                );
            })
            .unwrap()
    }
}
