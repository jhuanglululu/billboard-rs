//! [`Grid`]: a rectangle of block displays, laid out once and driven by cell.
//!
//! Heatmaps, tilemaps, pixel art, progress meters, sorting-visualiser rows —
//! every one of them is "W×H block displays on a regular pitch, recoloured from
//! data". Hand-rolling it is fifty lines of index arithmetic per animation, and
//! the arithmetic has one trap in it: a [`BlockDisplay`]'s position is its **low
//! corner**, not its centre (see that type's docs), so a tile smaller than the
//! cell pitch hugs the bottom-left-back of its cell and leaves all the slack at
//! the top. A grid laid out from cell centres looks half a tile too high.
//!
//! [`GridLayout`] does that arithmetic — centred cells, correct margins — and is
//! pure maths you can use on its own (for a caption, an overlay, a cell you
//! spawn yourself). [`Grid`] adds ownership: it holds the [`BlockDisplay`]s, so
//! dropping it despawns the whole sheet in one go.
//!
//! ```ignore
//! // A 16×9 heatmap, one block per sample, on a 0.5-block pitch with a small
//! // gap between tiles.
//! let layout = GridLayout::new(Position::new(0.0, 5.0, 0.0), 16, 9, 0.5, 0.45);
//! let mut sheet = Grid::spawn(layout, |col, row| palette.nearest(data[row][col]));
//!
//! // …a tick later, new data:
//! sheet.fill(|col, row| palette.nearest(data[row][col]));
//!
//! // …and a breath, without disturbing the layout:
//! sheet.pulse(1.15, Ticks::new(6));
//! ```
//!
//! # Orientation
//!
//! The sheet lies in the **XY plane** at the layout centre's `z`: columns run
//! along `+X` (column 0 leftmost when you look from `−Z`), rows run *down* `−Y`
//! (row 0 on top, so `data[row][col]` reads like a table). That is the one
//! orientation the helper knows; for a floor grid or a wall facing another way,
//! either rotate the tiles yourself, or use [`GridLayout`] for the maths and put
//! the results where you like. A grid is a plain pile of displays — nothing
//! stops you also adding its cells to a [`Group`](super::Group) and turning the
//! whole thing.

use crate::entity::{BlockDisplay, BlockState, WeakMut};
use crate::math::{Offset, Position, Scale, Ticks};

/// Where the cells of a `cols × rows` sheet sit. Pure maths, no entities.
///
/// The layout is defined from the **centre** of the whole sheet outwards, so
/// adding a column keeps the sheet centred instead of growing it to one side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLayout {
    /// The centre of the sheet — the middle of the middle cell for odd counts,
    /// the middle of the seam for even ones.
    pub center: Position,
    pub cols: usize,
    pub rows: usize,
    /// Distance between neighbouring cell centres, in blocks.
    pub pitch: f64,
    /// Edge length of one tile, as a scale on a 1×1×1 block. Anything less than
    /// `pitch` leaves a visible gap between neighbours — `tile = pitch * 0.9` is
    /// a tidy default; `tile = pitch` is a seamless sheet.
    pub tile: f64,
}

impl GridLayout {
    pub fn new(
        center: impl AsRef<Position>,
        cols: usize,
        rows: usize,
        pitch: f64,
        tile: f64,
    ) -> GridLayout {
        GridLayout {
            center: *center.as_ref(),
            cols,
            rows,
            pitch,
            tile,
        }
    }

    /// How many cells the sheet has.
    pub fn len(&self) -> usize {
        self.cols * self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The scale a tile is spawned at.
    pub fn tile_scale(&self) -> Scale {
        Scale::splat(self.tile)
    }

    /// The **geometric centre** of cell `(col, row)` — the point the tile is
    /// centred on, and where anything else that wants to sit "on" that cell
    /// belongs (a particle, a text display, a marker).
    ///
    /// Indices past the edge extrapolate rather than clamp: `cell_center(cols,
    /// 0)` is the cell just beyond the right edge, which is a convenient place
    /// to hang a legend.
    pub fn cell_center(&self, col: usize, row: usize) -> Position {
        let across = (col as f64 - (self.cols as f64 - 1.0) / 2.0) * self.pitch;
        let down = ((self.rows as f64 - 1.0) / 2.0 - row as f64) * self.pitch;
        self.center + Offset::new(across, down, 0.0)
    }

    /// The **spawn position** for cell `(col, row)`: the low corner a
    /// `tile`-sized block display needs so that its box ends up centred on
    /// [`cell_center`](GridLayout::cell_center).
    pub fn cell_position(&self, col: usize, row: usize) -> Position {
        GridLayout::centered(self.cell_center(col, row), self.tile_scale())
    }

    /// The low corner a block display of `scale` needs to end up centred on
    /// `center` — the half-tile correction, on its own, for anything you place
    /// beside a grid.
    ///
    /// ```ignore
    /// // A 0.6-block cube whose middle is exactly at eye level:
    /// BlockDisplay::spawn(block, GridLayout::centered(eye, Scale::splat(0.6)));
    /// ```
    pub fn centered(center: impl AsRef<Position>, scale: impl AsRef<Scale>) -> Position {
        let s = scale.as_ref();
        *center.as_ref() - Offset::new(s.x / 2.0, s.y / 2.0, s.z / 2.0)
    }

    /// Visible width of the sheet, edge to edge: the span from the left face of
    /// column 0 to the right face of the last column.
    pub fn width(&self) -> f64 {
        self.span(self.cols)
    }

    /// Visible height of the sheet, top face to bottom face.
    pub fn height(&self) -> f64 {
        self.span(self.rows)
    }

    fn span(&self, count: usize) -> f64 {
        if count == 0 {
            0.0
        } else {
            (count as f64 - 1.0) * self.pitch + self.tile
        }
    }

    /// A caption anchor: `gap` blocks below the sheet's bottom face, on its
    /// vertical centre line.
    ///
    /// This is the point to hand a [`TextDisplay`](crate::entity::TextDisplay),
    /// whose text is *centred* on its position — so the label's middle sits
    /// `gap` below the grid, not its top edge. Feed it to a block display and
    /// you would want [`centered`](GridLayout::centered) around it instead.
    pub fn caption_below(&self, gap: f64) -> Position {
        self.center - Offset::new(0.0, self.height() / 2.0 + gap, 0.0)
    }

    /// A caption anchor `gap` blocks above the sheet's top face.
    pub fn caption_above(&self, gap: f64) -> Position {
        self.center + Offset::new(0.0, self.height() / 2.0 + gap, 0.0)
    }
}

/// A `cols × rows` sheet of block displays, owned together.
///
/// The cells are stored row-major (`row * cols + col`) and the grid owns every
/// one of them: dropping it — or calling [`despawn`](Grid::despawn) — clears the
/// whole sheet, which is what makes a per-scene panel disposable with no
/// bookkeeping.
///
/// # Cost
///
/// [`spawn`](Grid::spawn) is **two** host calls per cell (spawn, then scale),
/// all in the tick you call it. [`fill`](Grid::fill) is one per cell — the SDK
/// caches nothing, so it resends every cell whether the data changed or not;
/// use [`set_block`](Grid::set_block) on the cells that moved if that matters.
/// [`move_to`](Grid::move_to) is one per cell, [`pulse`](Grid::pulse) and
/// [`resize`](Grid::resize) **two** (position and scale both move, because
/// scale grows from a display's corner and a centred tile has to be
/// re-anchored to stay centred).
///
/// A 16×9 sheet is 144 cells: fine to build, fine to refill, and worth a second
/// thought at 288 calls if you pulse it every tick. Handing a duration to
/// `pulse`/`move_to` and letting the client interpolate always beats driving
/// the grid yourself frame by frame.
pub struct Grid {
    layout: GridLayout,
    cells: Vec<BlockDisplay>,
}

impl Grid {
    /// Spawn every cell, asking `block` what belongs at each `(col, row)`.
    ///
    /// The whole sheet appears in this tick. For a staged reveal, spawn it and
    /// then grow it — `grid.pulse(0.0, Ticks::new(0))` followed by
    /// `grid.pulse(1.0, Ticks::new(20))` — or lay the cells out yourself from
    /// [`GridLayout::cell_position`] with sleeps in between.
    pub fn spawn<B: Into<BlockState>>(
        layout: GridLayout,
        mut block: impl FnMut(usize, usize) -> B,
    ) -> Grid {
        let mut cells = Vec::with_capacity(layout.len());
        for row in 0..layout.rows {
            for col in 0..layout.cols {
                let mut cell = BlockDisplay::spawn(block(col, row), layout.cell_position(col, row));
                cell.set_scale(layout.tile_scale());
                cells.push(cell);
            }
        }
        Grid { layout, cells }
    }

    /// The layout the cells were placed from.
    pub fn layout(&self) -> GridLayout {
        self.layout
    }

    pub fn cols(&self) -> usize {
        self.layout.cols
    }

    pub fn rows(&self) -> usize {
        self.layout.rows
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The display at `(col, row)`.
    ///
    /// Panics — which kills the animation, per the error philosophy — if the
    /// indices are outside the grid. An out-of-range cell is a bug in the
    /// caller's own arithmetic, not a condition to handle.
    pub fn cell(&self, col: usize, row: usize) -> &BlockDisplay {
        &self.cells[self.index(col, row)]
    }

    /// The display at `(col, row)`, mutably — for the per-cell attributes the
    /// grid has no opinion about (rotation, billboard mode, a one-off move).
    pub fn cell_mut(&mut self, col: usize, row: usize) -> &mut BlockDisplay {
        let i = self.index(col, row);
        &mut self.cells[i]
    }

    /// A weak reference to one cell, to hand to another task. The grid keeps
    /// ownership, so the cell dies with the grid.
    pub fn weak_mut(&self, col: usize, row: usize) -> WeakMut<BlockDisplay> {
        self.cell(col, row).weak_mut()
    }

    /// Every cell, in row-major order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BlockDisplay> {
        self.cells.iter_mut()
    }

    /// Set one cell's block. Instant — blocks cannot interpolate.
    pub fn set_block(&mut self, col: usize, row: usize, block: impl Into<BlockState>) {
        let i = self.index(col, row);
        self.cells[i].set_block(block);
    }

    /// Recolour the whole sheet from data: one call per cell, in row-major
    /// order. The redraw step of a heatmap.
    pub fn fill<B: Into<BlockState>>(&mut self, mut block: impl FnMut(usize, usize) -> B) {
        for row in 0..self.layout.rows {
            for col in 0..self.layout.cols {
                let i = row * self.layout.cols + col;
                self.cells[i].set_block(block(col, row));
            }
        }
    }

    /// Scale every tile to `factor` of its resting size over `over` ticks,
    /// keeping each one centred on its cell.
    ///
    /// The layout's `tile` is left alone, so `pulse(1.0, …)` always returns the
    /// sheet to rest — that is the breathe-and-relax move. `pulse(0.0, …)`
    /// shrinks it to nothing, which is how a panel leaves before it is dropped.
    ///
    /// Non-blocking, like every other `*_to`/`animate`: `sleep(over)` to wait
    /// it out.
    pub fn pulse(&mut self, factor: f64, over: Ticks) {
        self.apply_tile(self.layout.tile * factor, over);
    }

    /// Change the resting tile size — the sheet keeps its pitch, so this opens
    /// or closes the gaps between cells — and animate the cells into it.
    pub fn resize(&mut self, tile: f64, over: Ticks) {
        self.layout.tile = tile;
        self.apply_tile(tile, over);
    }

    /// Move the whole sheet to a new centre over `over` ticks, keeping its
    /// pitch and tile size. One host call per cell.
    pub fn move_to(&mut self, center: impl AsRef<Position>, over: Ticks) {
        self.layout.center = *center.as_ref();
        let layout = self.layout;
        for (i, cell) in self.cells.iter_mut().enumerate() {
            let (col, row) = (i % layout.cols, i / layout.cols);
            cell.move_to(layout.cell_position(col, row), over);
        }
    }

    /// Despawn the whole sheet now. Identical to dropping the grid; reads
    /// better at the end of a scene.
    pub fn despawn(self) {}

    /// Place and scale every cell for a tile size of `tile`, without touching
    /// the layout's own value. Position and scale both move: a display grows
    /// from its corner, so a tile that changes size must be re-anchored to stay
    /// centred on its cell.
    fn apply_tile(&mut self, tile: f64, over: Ticks) {
        let layout = self.layout;
        let scale = Scale::splat(tile);
        for (i, cell) in self.cells.iter_mut().enumerate() {
            let (col, row) = (i % layout.cols, i / layout.cols);
            let at = GridLayout::centered(layout.cell_center(col, row), scale);
            cell.move_to(at, over);
            cell.scale_to(scale, over);
        }
    }

    fn index(&self, col: usize, row: usize) -> usize {
        assert!(
            col < self.layout.cols && row < self.layout.rows,
            "grid cell ({col}, {row}) is outside a {}×{} grid",
            self.layout.cols,
            self.layout.rows
        );
        row * self.layout.cols + col
    }
}
