use engine::core::GameObject;
use engine::render::{Material, Mesh};
use engine::engine::IEngine;
use engine::render::Texture;
use engine::core::Aabb;

use super::{Metric, TextAlign};
use super::internal::ImguiState;

use std::fmt::Debug;
use engine::render::{MeshBuffer, MeshData, RenderQueue};
use engine::asset::Asset;

use std::rc::Rc;
use std::cell::RefCell;
use std::any::Any;
use std::cmp;

use na::Translation3;

pub trait Widget: Debug {
    fn id(&self) -> u32;
    fn bind(
        &self,
        ssize: (u32, u32),
        parent: &GameObject,
        engine: &mut IEngine,
    ) -> Rc<RefCell<GameObject>>;

    fn is_same(&self, other: &Widget) -> bool;
    fn as_any(&self) -> &Any;
}

impl PartialEq for Widget {
    fn eq(&self, other: &Widget) -> bool {
        self.is_same(other)
    }
}

fn make_text_mesh_data(s: &str, size: (u32, u32), hidpi: f32, align: TextAlign) -> MeshData {
    let mut vertices = vec![];
    let mut uvs = vec![];
    let mut indices = vec![];

    let icw = 8.0 / 128.0;
    let ich = 8.0 / 64.0;

    let mut i = 0;
    let nrow = 128 / 8;

    let gw = ((8 as f32) / size.0 as f32) * 2.0 * hidpi;
    let gh = ((8 as f32) / size.1 as f32) * 2.0 * hidpi;
    let mut base_y = 0.0;

    let lines: Vec<&str> = s.split('\n').collect();

    let max_len = lines.iter().fold(0, |acc, line| cmp::max(acc, line.len()));

    for line in lines.into_iter() {
        let x_offset = match align {
            TextAlign::Left => 0.0,
            TextAlign::Right => (max_len - line.len()) as f32 * gw,
            TextAlign::Center => (max_len - line.len()) as f32 * gw * 0.5,
        };

        for (cidx, c) in line.chars().enumerate() {
            let mut c: u8 = c as u8;
            if c >= 128 {
                c = 128;
            }

            let g_row = (c / nrow) as f32;
            let g_col = (c % nrow) as f32;

            let gx = (cidx as f32) * gw + x_offset;

            vertices.append(&mut vec![
                gx + 0.0, // 0
                base_y,
                0.0,
                gx + 0.0, // 1
                base_y - gh,
                0.0,
                gx + gw, // 2
                base_y - gh,
                0.0,
                gx + gw, // 3
                base_y,
                0.0,
            ]);

            uvs.append(&mut vec![
                g_col * icw + 0.0, // 0
                g_row * ich,
                g_col * icw + 0.0, // 1
                g_row * ich + ich,
                g_col * icw + icw, // 2
                g_row * ich + ich,
                g_col * icw + icw, // 3
                g_row * ich,
            ]);

            indices.append(&mut vec![
                i * 4,
                i * 4 + 1,
                i * 4 + 2,
                i * 4 + 0,
                i * 4 + 2,
                i * 4 + 3, // Top face
            ]);

            i += 1;
        }

        base_y -= gh * 2.0;
    }

    MeshData {
        vertices: vertices,
        uvs: Some(uvs),
        normals: None,
        indices: indices,
        tangents: None,
        bitangents: None,
    }
}

fn make_quad_mesh_data(size: (f32, f32)) -> MeshData {
    let w = size.0;
    let h = size.1;

    let vertices: Vec<f32> = vec![
            0.0, 0.0, 0.0,     // 0
            0.0, -h, 0.0,    // 1
            w, -h, 0.0,     // 2
            w, 0.0, 0.0       // 3
        ];

    let uvs: Vec<f32> = vec![
            // Top face
            0.0, 1.0,
            0.0, 0.0,
            1.0, 0.0,
            1.0, 1.0,
        ];

    let indices: Vec<u16> = vec![
        0, 1, 2, 0, 2, 3 // Top face
    ];

    MeshData {
        vertices: vertices,
        uvs: Some(uvs),
        normals: None,
        indices: indices,
        tangents: None,
        bitangents: None,
    }
}

fn compute_size_to_ndc(size: &Metric, ssize: &(u32, u32), hidpi: f32) -> (f32, f32) {
    let (x, y) = match size {
        &Metric::Native(px, py) => (px * 2.0, py * 2.0),
        &Metric::Pixel(px, py) => to_pixel_pos(px, py, ssize, hidpi),
        &Metric::Mixed((ax, ay), (bx, by)) => {
            let vp = to_pixel_pos(bx, by, ssize, hidpi);
            (ax * 2.0 + vp.0, ay * 2.0 + vp.1)
        }
    };

    return (x, y);
}

fn compute_translate(
    pos: &Metric,
    pivot: &Metric,
    ssize: &(u32, u32),
    hidpi: f32,
    bounds: &Aabb,
) -> Translation3<f32> {
    let w = bounds.max.x - bounds.min.x;
    let h = bounds.max.y - bounds.min.y;

    let (x, y) = match pos {
        &Metric::Native(px, py) => (px * 2.0, py * 2.0),
        &Metric::Pixel(px, py) => to_pixel_pos(px, py, ssize, hidpi),
        &Metric::Mixed((ax, ay), (bx, by)) => {
            let vp = to_pixel_pos(bx, by, ssize, hidpi);
            (ax * 2.0 + vp.0, ay * 2.0 + vp.1)
        }
    };

    let (offsetx, offsety) = match pivot {
        &Metric::Native(px, py) => (px * w, py * h),
        _ => unreachable!(),
    };

    Translation3::new(x - 1.0 - offsetx, y * -1.0 + 1.0 + offsety, 0.0)
}

fn to_pixel_pos(px: f32, py: f32, ssize: &(u32, u32), hidpi: f32) -> (f32, f32) {
    ((
        (px * 2.0 * hidpi) / (ssize.0 as f32),
        (py * 2.0 * hidpi) / (ssize.1 as f32),
    ))
}

#[derive(Debug, PartialEq)]
pub struct Label {
    id: u32,
    pos: Metric,
    state: ImguiState,
    s: String,
}

impl Label {
    pub fn new(id: u32, pos: Metric, state: ImguiState, s: String) -> Label {
        Self {
            id: id,
            pos: pos,
            state,
            s: s,
        }
    }
}

impl Widget for Label {
    fn id(&self) -> u32 {
        self.id
    }

    fn bind(
        &self,
        ssize: (u32, u32),
        parent: &GameObject,
        engine: &mut IEngine,
    ) -> Rc<RefCell<GameObject>> {
        let go = engine.new_game_object(parent);
        let db = engine.asset_system();

        {
            let hidpi = engine.hidpi_factor();
            let mut gomut = go.borrow_mut();
            let meshdata = make_text_mesh_data(&self.s, ssize, hidpi, self.state.text_align);

            let mut mesh = Mesh::new();
            let mut material = Material::new(db.new_program("default_ui"));
            material.set("uDiffuse", db.new_texture("default_font_bitmap"));
            material.render_queue = RenderQueue::UI;

            mesh.add_surface(MeshBuffer::new(meshdata), material);

            let mut gtran = gomut.transform.global();
            gtran.append_translation_mut(&compute_translate(
                &self.pos,
                &self.state.pivot,
                &ssize,
                hidpi,
                &mesh.bounds().unwrap().local_aabb(),
            ));
            gomut.transform.set_global(gtran);

            gomut.add_component(mesh);
        }

        go
    }

    fn is_same(&self, other: &Widget) -> bool {
        match other.as_any().downcast_ref::<Label>() {
            Some(o) => o == self,
            _ => false,
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}

#[derive(Debug)]
pub struct ImageRef<T: Debug>(Rc<T>);
impl<T: Debug> PartialEq for ImageRef<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, PartialEq)]
pub enum ImageKind {
    Texture(ImageRef<Texture>),
    Material(ImageRef<Material>),
}

impl From<Rc<Material>> for ImageKind {
    fn from(t: Rc<Material>) -> ImageKind {
        ImageKind::Material(ImageRef(t))
    }
}

impl From<Rc<Texture>> for ImageKind {
    fn from(t: Rc<Texture>) -> ImageKind {
        ImageKind::Texture(ImageRef(t))
    }
}

#[derive(Debug, PartialEq)]
pub struct Image {
    id: u32,
    pos: Metric,
    size: Metric,
    pivot: Metric,
    kind: ImageKind,
}

impl Image {
    pub fn new<T>(id: u32, pos: Metric, size: Metric, state: ImguiState, t: T) -> Image
    where
        T: Into<ImageKind>,
    {
        Self {
            id,
            pos,
            size,
            pivot: state.pivot,
            kind: t.into(),
        }
    }
    fn create_material(&self, engine: &mut IEngine) -> Rc<Material> {
        match self.kind {
            ImageKind::Material(ref m) => m.0.clone(),
            ImageKind::Texture(ref t) => {
                let db = engine.asset_system();

                let mut m = Material::new(db.new_program("default_ui"));
                m.render_queue = RenderQueue::UI;
                m.set("uDiffuse", t.0.clone());
                Rc::new(m)
            }
        }
    }
}

impl Widget for Image {
    fn id(&self) -> u32 {
        self.id
    }

    fn bind(
        &self,
        ssize: (u32, u32),
        parent: &GameObject,
        engine: &mut IEngine,
    ) -> Rc<RefCell<GameObject>> {
        let go = engine.new_game_object(parent);

        {
            let hidpi = engine.hidpi_factor();
            let mut gomut = go.borrow_mut();
            let meshdata = make_quad_mesh_data(compute_size_to_ndc(&self.size, &ssize, hidpi));

            let mut mesh = Mesh::new();
            let material = self.create_material(engine);
            mesh.add_surface(MeshBuffer::new(meshdata), material);

            let mut gtrans = gomut.transform.global();
            gtrans.append_translation_mut(&compute_translate(
                &self.pos,
                &self.pivot,
                &ssize,
                hidpi,
                &mesh.bounds().unwrap().local_aabb(),
            ));
            gomut.transform.set_global(gtrans);

            gomut.add_component(mesh);
        }

        go
    }

    fn is_same(&self, other: &Widget) -> bool {
        match other.as_any().downcast_ref::<Image>() {
            Some(o) => o == self,
            _ => false,
        }
    }

    fn as_any(&self) -> &Any {
        self
    }
}
