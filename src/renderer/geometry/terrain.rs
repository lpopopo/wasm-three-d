use crate::core::*;
use crate::renderer::*;
use std::rc::Rc;

const PATCH_SIZE: f32 = 16.0;
const PATCHES_PER_SIDE: u32 = 33;
const HALF_PATCHES_PER_SIDE: i32 = (PATCHES_PER_SIDE as i32 - 1) / 2;
const VERTICES_PER_UNIT: usize = 4;
const VERTICES_PER_SIDE: usize = (PATCH_SIZE + 1.0) as usize * VERTICES_PER_UNIT;
const VERTEX_DISTANCE: f32 = 1.0 / VERTICES_PER_UNIT as f32;

pub struct Terrain {
    context: Context,
    center: (i32, i32),
    patches: Vec<GroundPatch>,
    ground_material: Rc<PhysicalMaterial>,
}
impl Terrain {
    pub fn new(context: &Context, height_map: &impl Fn(f32, f32) -> f32, position: Vec3) -> Self {
        let mut loaded =
            three_d_asset::io::load(&["./assets/rocks_ground/rocks_ground_02_4k.gltf"]).unwrap();

        let ground_model: CpuModel = loaded.deserialize(".gltf").unwrap();
        let ground_material = Rc::new(PhysicalMaterial::new(context, &ground_model.materials[0]));
        let (x0, y0) = Self::pos2patch(position);
        let mut patches = Vec::new();
        for ix in x0 - HALF_PATCHES_PER_SIDE..x0 + HALF_PATCHES_PER_SIDE + 1 {
            for iy in y0 - HALF_PATCHES_PER_SIDE..y0 + HALF_PATCHES_PER_SIDE + 1 {
                let patch = GroundPatch::new(context, height_map, ix, iy);
                patches.push(patch);
            }
        }
        Self {
            context: context.clone(),
            center: (x0, y0),
            patches,
            ground_material,
        }
    }

    pub fn update(&mut self, position: Vec3, height_map: &impl Fn(f32, f32) -> f32) {
        let (x0, y0) = Self::pos2patch(position);

        while x0 > self.center.0 {
            self.center.0 += 1;
            for iy in
                self.center.1 - HALF_PATCHES_PER_SIDE..self.center.1 + HALF_PATCHES_PER_SIDE + 1
            {
                self.patches.push(GroundPatch::new(
                    &self.context,
                    height_map,
                    self.center.0 + HALF_PATCHES_PER_SIDE,
                    iy,
                ));
            }
        }

        while x0 < self.center.0 {
            self.center.0 -= 1;
            for iy in
                self.center.1 - HALF_PATCHES_PER_SIDE..self.center.1 + HALF_PATCHES_PER_SIDE + 1
            {
                self.patches.push(GroundPatch::new(
                    &self.context,
                    height_map,
                    self.center.0 - HALF_PATCHES_PER_SIDE,
                    iy,
                ));
            }
        }
        while y0 > self.center.1 {
            self.center.1 += 1;
            for ix in
                self.center.0 - HALF_PATCHES_PER_SIDE..self.center.0 + HALF_PATCHES_PER_SIDE + 1
            {
                self.patches.push(GroundPatch::new(
                    &self.context,
                    height_map,
                    ix,
                    self.center.1 + HALF_PATCHES_PER_SIDE,
                ));
            }
        }

        while y0 < self.center.1 {
            self.center.1 -= 1;
            for ix in
                self.center.0 - HALF_PATCHES_PER_SIDE..self.center.0 + HALF_PATCHES_PER_SIDE + 1
            {
                self.patches.push(GroundPatch::new(
                    &self.context,
                    height_map,
                    ix,
                    self.center.1 - HALF_PATCHES_PER_SIDE,
                ));
            }
        }

        self.patches.retain(|p| {
            (x0 - p.ix).abs() <= HALF_PATCHES_PER_SIDE && (y0 - p.iy).abs() <= HALF_PATCHES_PER_SIDE
        });
    }

    pub fn to_geometries(&self) -> Vec<&dyn Geometry> {
        self.patches
            .iter()
            .map(|p| p as &dyn Geometry)
            .collect::<Vec<_>>()
    }

    fn pos2patch(position: Vec3) -> (i32, i32) {
        (
            (position.x / PATCH_SIZE).floor() as i32,
            (position.z / PATCH_SIZE).floor() as i32,
        )
    }
}

struct GroundPatch {
    context: Context,
    ix: i32,
    iy: i32,
    index_buffer: ElementBuffer,
    coarse_index_buffer: ElementBuffer,
    very_coarse_index_buffer: ElementBuffer,
    positions_buffer: VertexBuffer,
    normals_buffer: VertexBuffer,
}

impl GroundPatch {
    fn new(context: &Context, height_map: &impl Fn(f32, f32) -> f32, ix: i32, iy: i32) -> Self {
        let offset = vec2(ix as f32 * PATCH_SIZE, iy as f32 * PATCH_SIZE);
        let positions = Self::positions(height_map, offset);
        let normals = Self::normals(height_map, offset, &positions);

        let positions_buffer = VertexBuffer::new_with_data(context, &positions);
        let normals_buffer = VertexBuffer::new_with_data(context, &normals);
        let index_buffer = ElementBuffer::new_with_data(context, &Self::indices(1));
        let coarse_index_buffer = ElementBuffer::new_with_data(context, &Self::indices(4));
        let very_coarse_index_buffer = ElementBuffer::new_with_data(context, &Self::indices(8));
        Self {
            context: context.clone(),
            ix,
            iy,
            index_buffer,
            coarse_index_buffer,
            very_coarse_index_buffer,
            positions_buffer,
            normals_buffer,
        }
    }

    fn index_buffer(&self, x0: i32, y0: i32) -> &ElementBuffer {
        let dist = (self.ix - x0).abs() + (self.iy - y0).abs();
        if dist > 4 {
            &self.very_coarse_index_buffer
        } else if dist > 8 {
            &self.coarse_index_buffer
        } else {
            &self.index_buffer
        }
    }

    fn indices(resolution: u32) -> Vec<u32> {
        let mut indices: Vec<u32> = Vec::new();
        let stride = VERTICES_PER_SIDE as u32;
        let max = (stride - 1) / resolution;
        for r in 0..max {
            for c in 0..max {
                indices.push(r * resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + (c * resolution + resolution) * stride);
                indices.push(r * resolution + resolution + c * resolution * stride);
                indices.push(r * resolution + resolution + (c * resolution + resolution) * stride);
            }
        }
        indices
    }

    fn positions(height_map: &impl Fn(f32, f32) -> f32, offset: Vec2) -> Vec<Vec3> {
        let mut data = vec![vec3(0.0, 0.0, 0.0); VERTICES_PER_SIDE * VERTICES_PER_SIDE];
        for r in 0..VERTICES_PER_SIDE {
            for c in 0..VERTICES_PER_SIDE {
                let vertex_id = r * VERTICES_PER_SIDE + c;
                let x = offset.x + r as f32 * VERTEX_DISTANCE;
                let z = offset.y + c as f32 * VERTEX_DISTANCE;
                data[vertex_id] = vec3(x, height_map(x, z), z);
            }
        }
        data
    }

    fn normals(
        height_map: &impl Fn(f32, f32) -> f32,
        offset: Vec2,
        positions: &Vec<Vec3>,
    ) -> Vec<Vec3> {
        let mut data = vec![vec3(0.0, 0.0, 0.0); VERTICES_PER_SIDE * VERTICES_PER_SIDE];
        let h = VERTEX_DISTANCE;
        for r in 0..VERTICES_PER_SIDE {
            for c in 0..VERTICES_PER_SIDE {
                let vertex_id = r * VERTICES_PER_SIDE + c;
                let x = offset.x + r as f32 * VERTEX_DISTANCE;
                let z = offset.y + c as f32 * VERTEX_DISTANCE;
                let xp = if r == VERTICES_PER_SIDE - 1 {
                    height_map(x + h, z)
                } else {
                    positions[vertex_id + VERTICES_PER_SIDE][1]
                };
                let xm = if r == 0 {
                    height_map(x - h, z)
                } else {
                    positions[vertex_id - VERTICES_PER_SIDE][1]
                };
                let zp = if c == VERTICES_PER_SIDE - 1 {
                    height_map(x, z + h)
                } else {
                    positions[vertex_id + 1][1]
                };
                let zm = if c == 0 {
                    height_map(x, z - h)
                } else {
                    positions[vertex_id - 1][1]
                };
                let dx = xp - xm;
                let dz = zp - zm;
                data[vertex_id] = vec3(-dx, 2.0 * h, -dz).normalize();
            }
        }
        data
    }
}

impl Geometry for GroundPatch {
    fn render_with_material(
        &self,
        material: &dyn Material,
        camera: &Camera,
        lights: &[&dyn Light],
    ) {
        let x0 = (camera.position().x / PATCH_SIZE).floor() as i32;
        let y0 = (camera.position().z / PATCH_SIZE).floor() as i32;
        let fragment_shader_source = material.fragment_shader_source(false, lights);
        self.context
            .program(
                &include_str!("shaders/terrain.vert"),
                &fragment_shader_source,
                |program| {
                    material.use_uniforms(program, camera, lights);
                    let transformation = Mat4::identity();
                    program.use_uniform("modelMatrix", &transformation);
                    program.use_uniform(
                        "viewProjectionMatrix",
                        &(camera.projection() * camera.view()),
                    );
                    program.use_uniform(
                        "normalMatrix",
                        &transformation.invert().unwrap().transpose(),
                    );
                    let render_states = RenderStates {
                        cull: Cull::Back,
                        ..Default::default()
                    };

                    program.use_vertex_attribute("position", &self.positions_buffer);
                    program.use_vertex_attribute("normal", &self.normals_buffer);

                    program.draw_elements(
                        render_states,
                        camera.viewport(),
                        &self.index_buffer(x0, y0),
                    );
                },
            )
            .unwrap();
    }

    fn aabb(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::new_with_positions(&[
            vec3(
                self.ix as f32 * PATCH_SIZE,
                -PATCH_SIZE,
                self.iy as f32 * PATCH_SIZE,
            ),
            vec3(
                (self.ix + 1) as f32 * PATCH_SIZE,
                PATCH_SIZE,
                (self.iy + 1) as f32 * PATCH_SIZE,
            ),
        ])
    }
}
