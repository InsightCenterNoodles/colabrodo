use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub enum Format {
    #[default]
    U8,
    U16,
    U32,

    U8VEC4,

    U16VEC2,

    VEC2,
    VEC3,
    VEC4,

    MAT3,
    MAT4,
}

pub type RGB = [f32; 3];
pub type RGBA = [f32; 4];

pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

pub type Mat3 = [f32; 9];
pub type Mat4 = [f32; 16];

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}
