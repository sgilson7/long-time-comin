//! STUB: replaced by the dioramas' commit.
use crate::content::Content;
use crate::geom::V3;
use crate::mesh::Mesh;
use crate::render::{boxed, Batch};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Open,
    Locked,
    Beaten,
    Sealed,
}

#[allow(clippy::too_many_arguments)]
pub fn diorama(
    b: &mut Batch,
    c: &Content,
    _zone: &str,
    at: V3,
    size: i32,
    _spin: i32,
    _hover: i32,
    _standing: Standing,
) {
    b.inked(
        Mesh::Sphere,
        &boxed(at, crate::geom::v3(size, size / 3, size)),
        c.pal.c("ink"),
        0,
    );
}
