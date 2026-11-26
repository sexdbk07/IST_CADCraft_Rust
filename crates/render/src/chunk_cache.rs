//! Display lists of model-space chunks kept between on-screen builds.
//!
//! The entity store keeps model space in copy-on-write chunks of up to 1024 objects, and an edit
//! copies only the chunks it touches. [`ChunkCache::build`] keeps each chunk's part of the display
//! list from the previous build and rebuilds only the chunks that are new since then. It then
//! joins the parts in drawing order, so the list is the same as a full [`crate::build`].
//! On a 26,000-object drawing an edit used to rebuild the whole list on the UI thread
//! (60-90 ms on two cores). Now it rebuilds one chunk.
//!
//! The cache keeps the drawing it last built from. That keeps every chunk of that drawing alive,
//! so a chunk address seen again always belongs to the same, unchanged chunk. Anything else that
//! changes how an object draws (the header, a table, a block, the build options) empties the
//! cache, and the next build is a full one.

use std::collections::HashMap;
use std::sync::Arc;

use super::display::DisplayList;
use super::pipeline::{Options, build};
#[cfg(not(target_arch = "wasm32"))]
use super::pipeline::{style_table, with_point_view};
use super::{Drawing, Space};

/// The parts of the last on-screen build of model space, by store chunk. See the module docs.
#[derive(Default)]
pub struct ChunkCache {
    /// The drawing the parts were built from, kept so its chunks stay alive.
    doc: Option<Arc<Drawing>>,
    /// The options the parts were built with (point sizes resolved).
    opts: Option<Options>,
    parts: HashMap<usize, DisplayList>,
    /// Chunks the last build took from the cache.
    pub reused: usize,
    /// Chunks the last build drew afresh.
    pub rebuilt: usize,
}

impl ChunkCache {
    /// Forget every part: the next build is a full one.
    pub fn clear(&mut self) {
        self.doc = None;
        self.opts = None;
        self.parts.clear();
        self.reused = 0;
        self.rebuilt = 0;
    }

    /// The display list of `space` in `doc`, like [`crate::build`]. Model space built for the
    /// screen reuses the parts of chunks unchanged since the last call. Any other build is a full
    /// one and empties the cache.
    pub fn build(&mut self, doc: &Arc<Drawing>, space: &Space, opts: &Options) -> DisplayList {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(list) = self.build_model(doc, space, opts) {
            return list;
        }
        self.clear();
        build(doc, space, opts)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn build_model(&mut self, doc: &Arc<Drawing>, space: &Space, opts: &Options) -> Option<DisplayList> {
        use super::display::append;
        use super::{MAX_BLOCK_EXPANSION, MAX_DISPLAY_POINTS};
        use rayon::prelude::*;
        if *space != Space::Model || opts.cancel.is_some() {
            return None;
        }
        let resolved = with_point_view(doc, space, opts);
        if style_table(doc, space, &resolved, false).is_some() {
            return None;
        }
        let same = self.doc.as_ref().is_some_and(|prev| same_context(prev, doc)) && self.opts.as_ref().is_some_and(|o| same_opts(o, &resolved));
        if !same {
            self.parts.clear();
        }
        let chunks: Vec<(usize, &[Arc<super::Entity>])> = doc.model.chunk_slices().collect();
        // Parts of chunks that are gone are dropped.
        let wanted: std::collections::HashSet<usize> = chunks.iter().map(|(k, _)| *k).collect();
        self.parts.retain(|k, _| wanted.contains(k));
        let missing: Vec<&(usize, &[Arc<super::Entity>])> = chunks.iter().filter(|(k, _)| !self.parts.contains_key(k)).collect();
        let (expanded, points) = self.parts.values().fold((0usize, 0usize), |(e, p), l| (e.saturating_add(l.expanded), p.saturating_add(l.verts.len() + l.tris.len())));
        let budget = std::sync::atomic::AtomicUsize::new(expanded);
        let charged = std::sync::atomic::AtomicUsize::new(points);
        let fresh: Vec<(usize, DisplayList)> = missing
            .par_iter()
            .map(|(key, items)| (*key, super::pipeline::build_chunk(doc, &resolved, items, &budget, &charged, MAX_DISPLAY_POINTS)))
            .collect();
        self.reused = chunks.len() - fresh.len();
        self.rebuilt = fresh.len();
        self.parts.extend(fresh);
        let total_points = self.parts.values().map(|l| l.verts.len() + l.tris.len()).sum::<usize>();
        let total_expanded = self.parts.values().map(|l| l.expanded).sum::<usize>();
        // A list cut short by a budget is built in one go, as a full build would cut it.
        if self.parts.values().any(|l| l.truncated) || total_points > MAX_DISPLAY_POINTS || total_expanded > MAX_BLOCK_EXPANSION {
            return None;
        }
        let mut list = DisplayList::default();
        list.prims.reserve(self.parts.values().map(|p| p.prims.len()).sum());
        list.verts.reserve(self.parts.values().map(|p| p.verts.len()).sum());
        list.tris.reserve(self.parts.values().map(|p| p.tris.len()).sum());
        for (key, _) in &chunks {
            if let Some(part) = self.parts.get(key) {
                append(&mut list, part.clone());
            }
        }
        self.doc = Some(doc.clone());
        self.opts = Some(resolved);
        Some(list)
    }
}

/// Whether two option sets draw an object the same way.
#[cfg(not(target_arch = "wasm32"))]
fn same_opts(a: &Options, b: &Options) -> bool {
    a.tolerance.to_bits() == b.tolerance.to_bits()
        && a.min_dash.to_bits() == b.min_dash.to_bits()
        && a.text == b.text
        && a.linetypes == b.linetypes
        && a.fill == b.fill
        && a.lineweights == b.lineweights
        && a.view_height.to_bits() == b.view_height.to_bits()
        && a.plot_style_table.is_none()
        && b.plot_style_table.is_none()
        && a.view == b.view
        && a.transparency == b.transparency
        && a.on_screen == b.on_screen
        && a.skip_viewport == b.skip_viewport
}

/// Whether everything but model space's objects that a model-space object draws with is the
/// same in both drawings: the header, the tables, the blocks (by pointer: a changed block is a
/// new copy), the loaded images and the annotation scales.
#[cfg(not(target_arch = "wasm32"))]
fn same_context(a: &Drawing, b: &Drawing) -> bool {
    a.header == b.header
        && a.layers == b.layers
        && a.linetypes == b.linetypes
        && a.text_styles == b.text_styles
        && a.dim_styles == b.dim_styles
        && a.mleader_styles == b.mleader_styles
        && a.table_styles == b.table_styles
        && a.mline_styles == b.mline_styles
        && a.scales == b.scales
        && a.hidden_objects == b.hidden_objects
        && a.portable_fonts == b.portable_fonts
        && a.proxy_layers == b.proxy_layers
        && a.refedit == b.refedit
        && a.parametric == b.parametric
        && Arc::ptr_eq(&a.foreign, &b.foreign)
        && a.rasters.same_as(&b.rasters)
        && a.blocks.len() == b.blocks.len()
        && a.blocks.iter().zip(&b.blocks).all(|((ka, va), (kb, vb))| ka == kb && Arc::ptr_eq(va, vb))
}
