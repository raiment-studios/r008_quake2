mod bounds;

pub mod prelude {
    pub use super::bounds::*;
}

use prelude::*;

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

use bevy::log::info;

#[derive(Debug)]
pub struct BSP38 {
    pub magic: String,
    pub version: u32,
    pub lumps: Vec<BSP38Lump>,
    pub bytes: Vec<u8>,

    bounds: Bounds,
}

#[derive(Debug)]
struct BSP38Lump {
    offset: i32,
    length: i32,
}

#[repr(u8)]
enum LumpIndex {
    Entities = 0,
    Planes = 1,
    Vertices = 2,
    Visibility = 3,
    Nodes = 4,
    Texinfo = 5,
    Faces = 6,
    Lighting = 7,
    Leafs = 8,
    LeafFaces = 9,
    LeafBrushes = 10,
    Edges = 11,
    FaceEdges = 12,
    Models = 13,
    Brushes = 14,
    BrushSides = 15,
    Pop = 16,
    Areas = 17,
    AreaPortals = 18,
    COUNT = 19,
}

#[derive(Debug)]
pub struct TextureInfo {
    pub u: [f32; 3],
    pub u0: f32,
    pub v: [f32; 3],
    pub v0: f32,
    pub flags: u32,
    pub value: u32,
    pub texture: String,
    pub next: u32,
}

#[derive(Debug)]
pub struct FaceData {
    pub points: Vec<f32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
    pub uv: Vec<f32>,
}

impl BSP38 {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut cursor = Cursor::new(&bytes);
        let magic = std::str::from_utf8(&bytes[0..4]).unwrap();
        cursor.set_position(4);
        if magic != "IBSP" {
            panic!("Invalid BSP38 file");
        }

        let version = cursor.read_u32::<LittleEndian>().unwrap();

        let mut lumps = Vec::with_capacity(LumpIndex::COUNT as usize);
        for _ in 0..LumpIndex::COUNT as usize {
            let lump = BSP38Lump {
                offset: cursor.read_i32::<LittleEndian>().unwrap(),
                length: cursor.read_i32::<LittleEndian>().unwrap(),
            };
            lumps.push(lump);
        }

        let mut bsp38 = BSP38 {
            magic: magic.to_string(),
            version,
            lumps,
            bytes,
            bounds: Bounds::default(),
        };
        bsp38.bounds = bsp38.compute_bounds();
        bsp38
    }

    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn compute_bounds(&self) -> Bounds {
        let mut bounds = Bounds::default();
        let vertices = self.read_vertices();
        for i in 0..vertices.len() / 3 {
            for j in 0..3 {
                bounds.min[j] = bounds.min[j].min(vertices[i * 3 + j]);
                bounds.max[j] = bounds.max[j].max(vertices[i * 3 + j]);
            }
        }
        bounds
    }

    // Convert the above JavaScript function to Rust
    pub fn read_vertices(&self) -> Vec<f32> {
        let lump = &self.lumps[LumpIndex::Vertices as usize];
        let num_vertices = lump.length as usize / 12;

        let mut cursor = Cursor::new(&self.bytes[lump.offset as usize..]);
        let mut buffer = Vec::with_capacity(3 * num_vertices);
        for _ in 0..num_vertices {
            buffer.push(cursor.read_f32::<LittleEndian>().unwrap());
            buffer.push(cursor.read_f32::<LittleEndian>().unwrap());
            buffer.push(cursor.read_f32::<LittleEndian>().unwrap());
        }
        buffer
    }

    // Returns all the edges in the BSP as a series of point pairs.
    //
    // The return buffer has 6 * N floats, where N is the number of edges.
    // The first 3 floats are the position of the edge start, and the next
    // 3 are the position of the edge end.
    pub fn read_edges(&self) -> Vec<f32> {
        let lump = &self.lumps[LumpIndex::Edges as usize];
        let num_edges = lump.length as usize / 4;

        let vertices = self.read_vertices();
        let mut cursor = Cursor::new(&self.bytes[lump.offset as usize..]);
        let mut buffer = Vec::with_capacity(6 * num_edges);
        for _ in 0..num_edges {
            let e0 = cursor.read_i16::<LittleEndian>().unwrap() as usize;
            let e1 = cursor.read_i16::<LittleEndian>().unwrap() as usize;
            for j in 0..3 {
                buffer.push(vertices[e0 * 3 + j]);
            }
            for j in 0..3 {
                buffer.push(vertices[e1 * 3 + j]);
            }
        }
        buffer
    }

    fn read_lump_as_cursor(&self, lump_index: LumpIndex) -> Cursor<&[u8]> {
        let lump = &self.lumps[lump_index as usize];
        Cursor::new(&self.bytes[lump.offset as usize..(lump.offset + lump.length) as usize])
    }

    pub fn read_face_edges(&self) -> Vec<i32> {
        let mut cursor = self.read_lump_as_cursor(LumpIndex::FaceEdges);
        let num_edges = cursor.get_ref().len() / 4;
        let mut buffer = Vec::with_capacity(num_edges);
        for _ in 0..num_edges {
            buffer.push(cursor.read_i32::<LittleEndian>().unwrap());
        }
        buffer
    }

    pub fn read_planes(&self) -> Vec<[f32; 4]> {
        const PLANE_SIZE: usize = 20;
        let mut cursor = self.read_lump_as_cursor(LumpIndex::Planes);
        let num_planes = cursor.get_ref().len() / PLANE_SIZE;
        let mut buffer = Vec::with_capacity(num_planes);
        for _ in 0..num_planes {
            let normal_x = cursor.read_f32::<LittleEndian>().unwrap();
            let normal_y = cursor.read_f32::<LittleEndian>().unwrap();
            let normal_z = cursor.read_f32::<LittleEndian>().unwrap();
            let distance = cursor.read_f32::<LittleEndian>().unwrap();
            let _type = cursor.read_u32::<LittleEndian>().unwrap();
            buffer.push([normal_x, normal_y, normal_z, distance]);
        }
        buffer
    }

    pub fn read_faces(&self) -> FaceData {
        let plane_data = self.read_planes();
        let face_edges = self.read_face_edges();
        let edge_data = self.read_edges();
        let tex_info = self.read_texture_info(); // Implement this similar to read_planes

        const FACE_BYTES: usize = 20;
        let lump = &self.lumps[LumpIndex::Faces as usize];
        let mut cursor = Cursor::new(&self.bytes[lump.offset as usize..]);
        let num_faces = lump.length as usize / FACE_BYTES;

        let mut positions = Vec::new();
        let mut uvs = Vec::new();
        let mut normals = Vec::new();
        let mut colors = Vec::new();

        for k in 0..num_faces {
            let offset = (k * FACE_BYTES) as u64;
            cursor.set_position(offset);

            let plane_index = cursor.read_u16::<LittleEndian>().unwrap() as usize;
            let plane_side = cursor.read_u16::<LittleEndian>().unwrap();
            let edge_index = cursor.read_u32::<LittleEndian>().unwrap() as usize;
            let edge_count = cursor.read_u16::<LittleEndian>().unwrap() as usize;
            let tex_index = cursor.read_u16::<LittleEndian>().unwrap() as usize;
            let _lightmap_styles = cursor.read_u32::<LittleEndian>().unwrap();
            let _lightmap_offset = cursor.read_u32::<LittleEndian>().unwrap();

            let mut normal = [
                plane_data[plane_index][0],
                plane_data[plane_index][1],
                plane_data[plane_index][2],
            ];
            if plane_side == 0 {
                normal.iter_mut().for_each(|n| *n = -*n);
            }

            let mut face_pts = Vec::new();
            for &fi in &face_edges[edge_index..edge_index + edge_count] {
                let (i0, i1) = if fi >= 0 {
                    ((fi as usize) * 6 + 0, (fi as usize) * 6 + 3)
                } else {
                    ((-fi as usize) * 6 + 3, (-fi as usize) * 6 + 6)
                };
                face_pts.push([edge_data[i0], edge_data[i0 + 1], edge_data[i0 + 2]]);
            }

            let palette = vec![
                [0.949, 0.6314, 0.5569],
                [0.3098, 0.7333, 0.7765],
                [0.9451, 0.6902, 0.1451],
                [0.7647, 0.8588, 0.3961],
                [0.7216, 0.1373, 0.0941],
                [0.4353, 0.5725, 0.1569],
                [0.4549, 0.1647, 0.0078],
                [0.5686, 0.3059, 0.6196],
                [0.2824, 0.4039, 0.1569],
            ];

            let tex = &tex_info[tex_index];

            for i in 2..face_pts.len() {
                let a = face_pts[0];
                let b = face_pts[i - 1];
                let c = face_pts[i];

                let wind = {
                    let cross = [
                        (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
                        (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
                        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
                    ];
                    let norm = (cross[0].powi(2) + cross[1].powi(2) + cross[2].powi(2)).sqrt();
                    [cross[0] / norm, cross[1] / norm, cross[2] / norm]
                };

                let tri = if wind.iter().zip(&normal).map(|(w, n)| w * n).sum::<f32>() < 0.0 {
                    [a, b, c]
                } else {
                    [c, b, a]
                };

                positions.extend_from_slice(&tri[0]);
                positions.extend_from_slice(&tri[1]);
                positions.extend_from_slice(&tri[2]);

                normals.extend_from_slice(&normal);
                normals.extend_from_slice(&normal);
                normals.extend_from_slice(&normal);

                for j in 0..3 {
                    let u = tex.u0 + tri[j].iter().zip(&tex.u).map(|(p, u)| p * u).sum::<f32>();
                    let v = tex.v0 + tri[j].iter().zip(&tex.v).map(|(p, v)| p * v).sum::<f32>();
                    uvs.push(u);
                    uvs.push(v);
                }

                let color = &palette[(k + i) % palette.len()];
                colors.extend_from_slice(color);
                colors.extend_from_slice(color);
                colors.extend_from_slice(color);
            }
        }

        // Points is a flattened array of 3d positions
        FaceData {
            points: positions,
            normals,
            colors,
            uv: uvs,
        }
    }

    pub fn read_texture_info(&self) -> Vec<TextureInfo> {
        const TEXTUREINFO_SIZE: usize = 76;
        let lump = &self.lumps[LumpIndex::Texinfo as usize];
        let mut cursor =
            Cursor::new(&self.bytes[lump.offset as usize..(lump.offset + lump.length) as usize]);
        let num_tex_info = lump.length as usize / TEXTUREINFO_SIZE;

        let mut buffer = Vec::with_capacity(num_tex_info);

        for _ in 0..num_tex_info {
            let u = [
                cursor.read_f32::<LittleEndian>().unwrap(),
                cursor.read_f32::<LittleEndian>().unwrap(),
                cursor.read_f32::<LittleEndian>().unwrap(),
            ];
            let u0 = cursor.read_f32::<LittleEndian>().unwrap();
            let v = [
                cursor.read_f32::<LittleEndian>().unwrap(),
                cursor.read_f32::<LittleEndian>().unwrap(),
                cursor.read_f32::<LittleEndian>().unwrap(),
            ];
            let v0 = cursor.read_f32::<LittleEndian>().unwrap();

            let flags = cursor.read_u32::<LittleEndian>().unwrap();
            let value = cursor.read_u32::<LittleEndian>().unwrap();

            let mut buf = Vec::new();
            for _ in 0..32 {
                buf.push(cursor.read_u8().unwrap());
            }
            let texture = buf
                .iter()
                .filter(|&&b| b != 0)
                .map(|&b| b as char)
                .collect::<String>()
                .trim()
                .to_string();

            let next = cursor.read_u32::<LittleEndian>().unwrap();

            buffer.push(TextureInfo {
                u,
                u0,
                v,
                v0,
                flags,
                value,
                texture,
                next,
            });
        }

        buffer
    }
}
