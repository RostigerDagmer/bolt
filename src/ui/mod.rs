pub mod text;
use std::{sync::Arc, path::PathBuf};

pub use text::*;

use crate::Context;



pub struct UI {
    pub text: Vec<Text>,
    pub atlas: GlyphAtlas,
}

impl UI {
    pub fn new(context: Arc<Context>) -> Self {
        Self {
            text: Vec::new(),
            atlas: ftt_to_atlas(context.clone(), &PathBuf::from("fonts/Montserrat/Montserrat-VariableFont_wght.ttf")),
        }
    }

    pub fn add_text(&mut self, text: Text) {
        self.text.push(text);
    }

    pub fn clear_text(&mut self) {
        self.text.clear();
    }

    // constructs an array of glyphs for each text
    pub fn glyphs(&self) -> Vec<GlyphInstance> {

        let mut glyphs: Vec<GlyphInstance> = Vec::new();
        self.text.iter().for_each(|text| {
            // for each character in the text
            let mut off_x = 0.0;
            text.text.chars().for_each(|c| {
                // get the glyph from the atlas
                let glyph = self.atlas.glyphs.get(&c).unwrap();
                off_x += glyph.advance;
                // construct the glyph instance with individual transform
                glyphs.push(GlyphInstance {
                    wh_atlas: glam::Vec4::new(glyph.width, glyph.height, glyph.atlas_x, glyph.atlas_y),
                    transform: text.transform * glam::Mat4::from_translation(glam::Vec3::new(off_x, 0.0, 0.0)),
                    color: text.color,
                });
            })
        });
        glyphs
    }
}

pub struct Text {
    pub text: String,
    pub font_size: f32,
    pub color: glam::Vec4,
    pub transform: glam::Mat4,
}