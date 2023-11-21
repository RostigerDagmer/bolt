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
        let fontface = self.atlas.font_face().unwrap();
        let kerning_table = self.atlas.kerning_table();
        let font_height = fontface.height() as f32;
        let font_width = fontface.width().to_number() as f32;
        let glyph_res = 64.0; // todo: move into atlas

        self.text.iter().for_each(|text| {
            // for each character in the text
            let mut off_x = 0.0;
            // calculate the normalized font scaling
            let scale = text.font_size * glyph_res / font_height ;

            text.text.chars().collect::<Vec<_>>().windows(2).for_each(|c| {
                // get the glyph from the atlas
                let c1 = c[0];
                let c2 = c.get(1).cloned().unwrap_or(' ');
                let glyph = self.atlas.glyphs.get(&c1).unwrap();
                let kerning = match kerning_table {
                    Some(table) => {
                        find_kerning_of_pair(fontface.clone(), table, &c1, &c2)
                    }
                    None => {
                        0.0
                    }
                };
                off_x += glyph.advance;
                
                // calculate the normalized height
                let norm_height = glyph.height / font_height * scale;
                // calculate the normalized width
                let norm_width = glyph.width / font_height * scale;

                // calculate the normalized offset
                let norm_off_x = ((off_x + kerning) / font_width) / scale;

                // calculate the baseline offset
                let baseline_off = (fontface.ascender() as f32 / font_height) * scale;

                // construct the glyph instance with individual transform
                let transform = text.transform * glam::Mat4::from_translation(glam::Vec3::new(norm_off_x, baseline_off, 0.0));
                let scale = glam::Mat4::from_scale(glam::Vec3::new(norm_width, -norm_height, 1.0));

                glyphs.push(GlyphInstance {
                    wh_atlas: glam::Vec4::new(glyph.width, glyph.height, glyph.atlas_x, glyph.atlas_y),
                    transform: text.transform * transform * scale,
                    color: text.color,
                });
            })
        });
        glyphs
    }
}

pub fn find_kerning_of_pair(fontface: ttf_parser::Face, table: ttf_parser::kern::Table, left: &char, right: &char) -> f32 {
    let mut kerning = 0.0;
    table.subtables.into_iter().for_each(|subtable| {
        match subtable.format {
            ttf_parser::kern::Format::Format0(table) => {
                let l_id = fontface.glyph_index(*left).unwrap();
                let r_id = fontface.glyph_index(*right).unwrap();
                kerning = table.glyphs_kerning(l_id, r_id).unwrap_or(0) as f32;
            }
            _ => {}
        }
    });
    kerning
}

pub struct Text {
    pub text: String,
    pub font_size: f32,
    pub color: glam::Vec4,
    pub transform: glam::Mat4,
}