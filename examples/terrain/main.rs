// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate cgmath;
#[macro_use]
extern crate gfx;
extern crate gfx_app;
extern crate time;
extern crate rand;
extern crate genmesh;
extern crate noise;

use rand::Rng;
use cgmath::{SquareMatrix, Matrix4, Point3, Vector3};
use cgmath::{Transform, AffineMatrix3};
pub use gfx::format::{DepthStencil};
pub use gfx_app::ColorFormat;
use genmesh::{Vertices, Triangulate};
use genmesh::generators::{Plane, SharedVertex, IndexedPolygon};
use time::precise_time_s;
use noise::{Seed, perlin2};


gfx_vertex_struct!( Vertex {
    pos: [f32; 3] = "a_Pos",
    color: [f32; 3] = "a_Color",
});

gfx_constant_struct!( Locals {
    model: [[f32; 4]; 4] = "u_Model",
    view: [[f32; 4]; 4] = "u_View",
    proj: [[f32; 4]; 4] = "u_Proj",
});

gfx_pipeline!(pipe {
    vbuf: gfx::VertexBuffer<Vertex> = (),
    locals: gfx::ConstantBuffer<Locals> = "Locals",
    model: gfx::Global<[[f32; 4]; 4]> = "u_Model",
    view: gfx::Global<[[f32; 4]; 4]> = "u_View",
    proj: gfx::Global<[[f32; 4]; 4]> = "u_Proj",
    out_color: gfx::RenderTarget<ColorFormat> = "Target0",
    out_depth: gfx::DepthTarget<DepthStencil> =
        gfx::preset::depth::LESS_EQUAL_WRITE,
});

fn calculate_color(height: f32) -> [f32; 3] {
    if height > 8.0 {
        [0.9, 0.9, 0.9] // white
    } else if height > 0.0 {
        [0.7, 0.7, 0.7] // greay
    } else if height > -5.0 {
        [0.2, 0.7, 0.2] // green
    } else {
        [0.2, 0.2, 0.7] // blue
    }
}

struct App<R: gfx::Resources> {
    pso: gfx::PipelineState<R, pipe::Meta>,
    data: pipe::Data<R>,
    slice: gfx::Slice<R>,
}

impl<R: gfx::Resources> gfx_app::Application<R> for App<R> {
    fn new<F: gfx::Factory<R>>(mut factory: F, init: gfx_app::Init<R>) -> Self {
        use gfx::traits::FactoryExt;

        let vs = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/terrain_120.glslv"),
            glsl_150: include_bytes!("shader/terrain_150.glslv"),
            hlsl_40:  include_bytes!("data/vertex.fx"),
            .. gfx_app::shade::Source::empty()
        };
        let ps = gfx_app::shade::Source {
            glsl_120: include_bytes!("shader/terrain_120.glslf"),
            glsl_150: include_bytes!("shader/terrain_150.glslf"),
            hlsl_40:  include_bytes!("data/pixel.fx"),
            .. gfx_app::shade::Source::empty()
        };

        let rand_seed = rand::thread_rng().gen();
        let seed = Seed::new(rand_seed);
        let plane = Plane::subdivide(256, 256);
        let vertex_data: Vec<Vertex> = plane.shared_vertex_iter()
            .map(|(x, y)| {
                let h = perlin2(&seed, &[x, y]) * 32.0;
                Vertex {
                    pos: [25.0 * x, 25.0 * y, h],
                    color: calculate_color(h),
                }
            })
            .collect();

        let index_data: Vec<u32> = plane.indexed_polygon_iter()
            .triangulate()
            .vertices()
            .map(|i| i as u32)
            .collect();

        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, &index_data[..]);

        App {
            pso: factory.create_pipeline_simple(
                vs.select(init.backend).unwrap(),
                ps.select(init.backend).unwrap(),
                gfx::state::CullFace::Back,
                pipe::new()
                ).unwrap(),
            data: pipe::Data {
                vbuf: vbuf,
                locals: factory.create_constant_buffer(1),
                model: Matrix4::identity().into(),
                view: Matrix4::identity().into(),
                proj: cgmath::perspective(
                    cgmath::deg(60.0f32), init.aspect_ratio, 0.1, 1000.0
                    ).into(),
                out_color: init.color,
                out_depth: init.depth,
            },
            slice: slice,
        }
    }

    fn render<C: gfx::CommandBuffer<R>>(&mut self, encoder: &mut gfx::Encoder<R, C>) {
        let time = precise_time_s() as f32;
        let x = time.sin();
        let y = time.cos();
        let view: AffineMatrix3<f32> = Transform::look_at(
            Point3::new(x * 32.0, y * 32.0, 16.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::unit_z(),
        );

        self.data.view = view.mat.into();
        let locals = Locals {
            model: self.data.model,
            view: self.data.view,
            proj: self.data.proj,
        };

        encoder.update_buffer(&self.data.locals, &[locals], 0).unwrap();
        encoder.clear(&self.data.out_color, [0.3, 0.3, 0.3, 1.0]);
        encoder.clear_depth(&self.data.out_depth, 1.0);
        encoder.draw(&self.slice, &self.pso, &self.data);
    }
}

pub fn main() {
    use gfx_app::Application;
    App::launch_default("Terrain example");
}
