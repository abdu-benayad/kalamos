/// Current cursor location
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct Cursor {
    /// Index of [`BufferLine`](crate::BufferLine) in [`Buffer::lines`](crate::Buffer::lines)
    pub line: usize,
    /// First-byte-index of glyph at cursor (will insert behind this glyph)
    pub index: usize,
    /// Whether to associate the cursor with the run before it or the run after it if placed at the
    /// boundary between two runs
    pub affinity: Affinity,
}

impl Cursor {
    /// Create a new cursor
    pub const fn new(line: usize, index: usize) -> Self {
        Self::new_with_affinity(line, index, Affinity::Before)
    }

    /// Create a new cursor, specifying the affinity
    pub const fn new_with_affinity(line: usize, index: usize, affinity: Affinity) -> Self {
        Self {
            line,
            index,
            affinity,
        }
    }
}

/// Whether to associate cursors placed at a boundary between runs with the run before or after it.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Affinity {
    #[default]
    Before,
    After,
}

impl Affinity {
    pub fn before(&self) -> bool {
        *self == Self::Before
    }

    pub fn after(&self) -> bool {
        *self == Self::After
    }

    pub const fn from_before(before: bool) -> Self {
        if before {
            Self::Before
        } else {
            Self::After
        }
    }

    pub const fn from_after(after: bool) -> Self {
        if after {
            Self::After
        } else {
            Self::Before
        }
    }
}

/// The position of a cursor within a [`Buffer`](crate::Buffer).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LayoutCursor {
    /// Index of [`BufferLine`](crate::BufferLine) in [`Buffer::lines`](crate::Buffer::lines)
    pub line: usize,
    /// Index of [`LayoutLine`](crate::LayoutLine) in [`BufferLine::layout`](crate::BufferLine::layout)
    pub layout: usize,
    /// Index of [`LayoutGlyph`](crate::LayoutGlyph) in [`LayoutLine::glyphs`](crate::LayoutLine::glyphs)
    pub glyph: usize,
}

impl LayoutCursor {
    /// Create a new [`LayoutCursor`]
    pub const fn new(line: usize, layout: usize, glyph: usize) -> Self {
        Self {
            line,
            layout,
            glyph,
        }
    }
}

/// A motion to perform on a [`Cursor`]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Motion {
    /// Apply specific [`LayoutCursor`]
    LayoutCursor(LayoutCursor),
    /// Move cursor to previous character ([`Self::Left`] in LTR, [`Self::Right`] in RTL)
    Previous,
    /// Move cursor to next character ([`Self::Right`] in LTR, [`Self::Left`] in RTL)
    Next,
    /// Move cursor left
    Left,
    /// Move cursor right
    Right,
    /// Move cursor up
    Up,
    /// Move cursor down
    Down,
    /// Move cursor to start of line
    Home,
    /// Move cursor to start of line, skipping whitespace
    SoftHome,
    /// Move cursor to end of line
    End,
    /// Move cursor to start of paragraph
    ParagraphStart,
    /// Move cursor to end of paragraph
    ParagraphEnd,
    /// Move cursor up one page
    PageUp,
    /// Move cursor down one page
    PageDown,
    /// Move cursor up or down by a number of pixels
    Vertical(i32),
    /// Move cursor to previous word boundary
    PreviousWord,
    /// Move cursor to next word boundary
    NextWord,
    /// Move cursor to next word boundary to the left
    LeftWord,
    /// Move cursor to next word boundary to the right
    RightWord,
    /// Move cursor to the start of the document
    BufferStart,
    /// Move cursor to the end of the document
    BufferEnd,
    /// Move cursor to specific line
    GotoLine(usize),
}

/// Scroll position in [`Buffer`](crate::Buffer)
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Scroll {
    /// Index of [`BufferLine`](crate::BufferLine) in [`Buffer::lines`](crate::Buffer::lines). This will be adjusted as needed if layout is
    /// out of bounds
    pub line: usize,
    /// Pixel offset from the start of the [`BufferLine`](crate::BufferLine). This will be adjusted as needed
    /// if it is negative or exceeds the height of the [`BufferLine::layout`](crate::BufferLine::layout) lines.
    pub vertical: f32,
    /// The horizontal position of scroll in fractional pixels.
    ///
    /// The buffer only *maintains* this value: the cursor-visibility pass in
    /// [`Buffer::shape_until_cursor`](crate::Buffer::shape_until_cursor)
    /// adjusts it so the cursor stays inside the viewport, but no in-crate
    /// layout or draw path applies it — glyph positions are unshifted.
    /// Offsetting painted glyphs by `-horizontal` is the embedder's job.
    pub horizontal: f32,
}

impl Scroll {
    /// Create a new scroll
    pub const fn new(line: usize, vertical: f32, horizontal: f32) -> Self {
        Self {
            line,
            vertical,
            horizontal,
        }
    }
}
