use bevy::prelude::*;
use big_space::{BigSpaceCommands, GridCell, ReferenceFrame};

pub type Precision = i32;
pub type Grid = GridCell<Precision>;

const CELL_LENGTH: f32 = 10_000.0;

pub fn reference_frame() -> ReferenceFrame<Precision> {
    ReferenceFrame::new(CELL_LENGTH, 100f32)
}