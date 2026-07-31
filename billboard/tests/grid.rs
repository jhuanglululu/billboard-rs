//! Known-answer tests for [`GridLayout`]'s placement arithmetic.
//!
//! Every expectation is worked out by hand from the two rules the layout is:
//!
//! ```text
//! cell_center(col, row) = center + ( (col - (cols-1)/2) * pitch,
//!                                    ((rows-1)/2 - row) * pitch,
//!                                    0 )
//! cell_position(...)    = cell_center(...) - (tile/2, tile/2, tile/2)
//! ```
//!
//! The second line is the whole point of the helper: a block display's position
//! is its low corner, so a tile has to be pulled back half its own size on
//! every axis to end up centred on its cell.
//!
//! Pure guest maths — no entities, so no ABI. [`Grid`](billboard::helpers::Grid)
//! itself spawns displays and cannot be exercised without a host: `spawn`,
//! `fill`/`fill_row`/`fill_col`, `pulse`/`pulse_cell`/`pulse_row`/`pulse_col`,
//! `resize` and `move_to` all drive real entities through host calls that are
//! panicking stubs off-target. What *is* pinned here is the arithmetic they
//! feed those calls — `cell_center` and `centered`, which is where the
//! re-anchor dance those methods perform comes from.

use billboard::helpers::GridLayout;
use billboard::math::{Offset, Position, Scale};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_pos(got: Position, x: f64, y: f64, z: f64) {
    assert!(
        approx(got.x, x) && approx(got.y, y) && approx(got.z, z),
        "expected position ({x}, {y}, {z}), got {got:?}"
    );
}

/// A 3×3 sheet on a 1-block pitch, centred on the origin, tiles filling their
/// cells: cell centres are the integer lattice −1, 0, +1 on both axes.
fn three_by_three() -> GridLayout {
    GridLayout::new(Position::ZERO, 3, 3, 1.0, 1.0)
}

#[test]
fn odd_grids_put_the_middle_cell_on_the_center() {
    let g = three_by_three();
    assert_pos(g.cell_center(1, 1), 0.0, 0.0, 0.0);
    // Column 0 is one pitch west, column 2 one pitch east.
    assert_pos(g.cell_center(0, 1), -1.0, 0.0, 0.0);
    assert_pos(g.cell_center(2, 1), 1.0, 0.0, 0.0);
    // Row 0 is the TOP row: +1 on y, and row 2 is −1.
    assert_pos(g.cell_center(1, 0), 0.0, 1.0, 0.0);
    assert_pos(g.cell_center(1, 2), 0.0, -1.0, 0.0);
    // Corners compose the two.
    assert_pos(g.cell_center(0, 0), -1.0, 1.0, 0.0);
    assert_pos(g.cell_center(2, 2), 1.0, -1.0, 0.0);
}

#[test]
fn even_grids_straddle_the_center() {
    // 4 columns on a 2-block pitch: centres at −3, −1, +1, +3, so the sheet's
    // middle falls on the seam between columns 1 and 2.
    let g = GridLayout::new(Position::ZERO, 4, 1, 2.0, 2.0);
    assert_pos(g.cell_center(0, 0), -3.0, 0.0, 0.0);
    assert_pos(g.cell_center(1, 0), -1.0, 0.0, 0.0);
    assert_pos(g.cell_center(2, 0), 1.0, 0.0, 0.0);
    assert_pos(g.cell_center(3, 0), 3.0, 0.0, 0.0);
}

#[test]
fn cell_position_pulls_the_tile_back_half_its_size() {
    // Tiles smaller than the pitch: 0.5 wide on a 1.0 pitch, so each tile is
    // centred in its cell with 0.25 of slack on either side.
    let g = GridLayout::new(Position::new(10.0, 4.0, -2.0), 3, 3, 1.0, 0.5);
    // Middle cell: centre is the layout centre, corner is 0.25 below it on
    // every axis.
    assert_pos(g.cell_center(1, 1), 10.0, 4.0, -2.0);
    assert_pos(g.cell_position(1, 1), 9.75, 3.75, -2.25);
    // Top-left cell: centre (9, 5, −2), corner 0.25 back from that.
    assert_pos(g.cell_center(0, 0), 9.0, 5.0, -2.0);
    assert_pos(g.cell_position(0, 0), 8.75, 4.75, -2.25);
}

#[test]
fn a_full_pitch_tile_makes_a_seamless_sheet() {
    // tile == pitch: neighbouring tiles touch. Cell (0,0)'s corner is at −1.5,
    // cell (1,0)'s at −0.5 — exactly one tile width apart, no gap, no overlap.
    let g = three_by_three();
    assert_pos(g.cell_position(0, 0), -1.5, 0.5, -0.5);
    assert_pos(g.cell_position(1, 0), -0.5, 0.5, -0.5);
    assert_pos(g.cell_position(2, 0), 0.5, 0.5, -0.5);
}

#[test]
fn centered_is_the_half_box_correction_on_its_own() {
    // A non-uniform box: pulled back half of each axis independently.
    let at = GridLayout::centered(Position::new(1.0, 2.0, 3.0), Scale::new(0.8, 4.0, 0.2));
    assert_pos(at, 0.6, 0.0, 2.9);
    // The identity case: a unit block centred on a point sits half a block back.
    let unit = GridLayout::centered(Position::ZERO, Scale::splat(1.0));
    assert_pos(unit, -0.5, -0.5, -0.5);
}

#[test]
fn extent_spans_face_to_face_not_centre_to_centre() {
    // 3 cells on a 1.0 pitch with 0.5 tiles: outer centres are 2.0 apart, and
    // the visible sheet runs half a tile further at each end — 2.5.
    let g = GridLayout::new(Position::ZERO, 3, 3, 1.0, 0.5);
    assert!(approx(g.width(), 2.5));
    assert!(approx(g.height(), 2.5));
    // Seamless: 3 cells of 1.0 on a 1.0 pitch are exactly 3 blocks wide.
    assert!(approx(three_by_three().width(), 3.0));
    // A single cell is just the tile; an empty axis is nothing.
    assert!(approx(
        GridLayout::new(Position::ZERO, 1, 1, 1.0, 0.4).width(),
        0.4
    ));
    assert!(approx(
        GridLayout::new(Position::ZERO, 0, 0, 1.0, 0.4).width(),
        0.0
    ));
}

#[test]
fn captions_clear_the_visible_edge_by_the_gap() {
    // Height is 2.5 (above), so the bottom face is 1.25 below the centre y of
    // 6.0, and a 0.25 gap puts the caption anchor at 4.5.
    let g = GridLayout::new(Position::new(0.0, 6.0, 1.0), 3, 3, 1.0, 0.5);
    assert_pos(g.caption_below(0.25), 0.0, 4.5, 1.0);
    assert_pos(g.caption_above(0.25), 0.0, 7.5, 1.0);
    // Zero gap lands exactly on the face.
    assert_pos(g.caption_below(0.0), 0.0, 4.75, 1.0);
}

#[test]
fn tile_scale_and_len_report_the_layout() {
    let g = GridLayout::new(Position::ZERO, 16, 9, 0.5, 0.45);
    assert_eq!(g.len(), 144);
    assert!(!g.is_empty());
    assert!(GridLayout::new(Position::ZERO, 0, 9, 0.5, 0.45).is_empty());
    let s = g.tile_scale();
    assert!(approx(s.x, 0.45) && approx(s.y, 0.45) && approx(s.z, 0.45));
}

#[test]
fn sized_is_new_at_the_origin() {
    // Same shape, centre at ZERO — so every cell centre is the offset from the
    // sheet's middle, which is what makes it worth measuring before placing.
    let g = GridLayout::sized(3, 3, 1.0, 1.0);
    assert_eq!(g, GridLayout::new(Position::ZERO, 3, 3, 1.0, 1.0));
    assert_pos(g.cell_center(1, 1), 0.0, 0.0, 0.0);
    assert_pos(g.cell_center(0, 0), -1.0, 1.0, 0.0);
}

#[test]
fn extent_does_not_depend_on_the_centre() {
    // The premise of measure-then-place: width/height come from the shape
    // alone, so a layout can be measured before anyone decides where it goes.
    let at_origin = GridLayout::sized(4, 8, 0.5, 0.45);
    let mut moved = at_origin;
    moved.center = Position::new(-17.0, 62.5, 3.25);
    assert!(approx(at_origin.width(), moved.width()));
    assert!(approx(at_origin.height(), moved.height()));
    // Worked by hand: 4 cells on a 0.5 pitch spans 3 * 0.5 + 0.45 = 1.95, and
    // 8 rows spans 7 * 0.5 + 0.45 = 3.95.
    assert!(approx(at_origin.width(), 1.95));
    assert!(approx(at_origin.height(), 3.95));
}

#[test]
fn measuring_then_placing_puts_a_gap_between_two_sheets_faces() {
    // The pattern from the module docs: B sits left of A with exactly 1.0 of
    // air between their facing edges.
    let mut a = GridLayout::sized(4, 8, 0.5, 0.45); // width 1.95
    let mut b = GridLayout::sized(8, 12, 0.5, 0.45); // width 3.95
    a.center = Position::new(10.0, 5.0, 0.0);
    b.center = a.center - Offset::new(a.width() / 2.0 + 1.0 + b.width() / 2.0, 0.0, 0.0);

    // A's left face, B's right face, and the gap between them.
    let a_left = a.center.x - a.width() / 2.0;
    let b_right = b.center.x + b.width() / 2.0;
    assert!(approx(a_left - b_right, 1.0));
    // Sanity on the absolute number: 10 − (0.975 + 1.0 + 1.975) = 6.05.
    assert!(approx(b.center.x, 6.05));
}
