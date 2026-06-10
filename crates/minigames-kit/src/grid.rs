//! Shared inline styling for the kit's CRT grid minigames (Hunter, BucketBrigade,
//! …). The layout is set inline rather than via Tailwind utilities because some
//! hosts (the Tailwind Play CDN) emit grid/flex utilities only lazily, which would
//! shift a centered grid mid-run; inline styles are stable from first paint.

/// The grid container: a centered, dim-bordered lattice `cols` cells wide. A game
/// can append its own declarations (e.g. `touch-action`) after this.
pub fn container_style(cols: usize) -> String {
    format!(
        "display: grid; width: 100%; max-width: 19rem; \
         grid-template-columns: repeat({cols}, 1fr); gap: 2px; \
         padding: 4px; border: 1px solid var(--phosphor-dim); border-radius: 4px; \
         background: rgba(20, 95, 40, 0.10);"
    )
}

/// The shared per-cell box. `line-height: 1` + `overflow: hidden` pin the cell to
/// its aspect ratio so a glyph can't grow the row and make the grid jitter as the
/// board changes. Each game appends its own `border`/`background`/`cursor` after
/// this base.
pub const CELL_BOX_STYLE: &str = "display: flex; align-items: center; justify-content: center; \
     aspect-ratio: 1; font-size: 1.35rem; font-weight: 700; \
     line-height: 1; overflow: hidden; border-radius: 3px;";
