

use std::{sync::Arc, path::PathBuf};

use bolt::{AppSettings, Window, Context, RendererSettings, SharedContext, AppRenderer, ui::ftt_to_atlas};
use fontdue::Metrics;
use harfbuzz_rs::Font;
use image::{DynamicImage, GenericImage};
use sdf_glyph_renderer::{BitmapGlyph};
use ttf_parser::Face;
use winit::event_loop::EventLoop;


pub fn fft_parser_impl() {
    let fontfile = bolt::util::find_asset("fonts/Montserrat/Montserrat-VariableFont_wght.ttf");
    if fontfile.is_none() {
        panic!("Could not find font file");
    }
    let path = fontfile.unwrap();

    // load binary data from path to font file
    let font_data = std::fs::read(path).unwrap();

    let face: Face = Face::parse(&font_data, 0).unwrap();
    let glyph_index = face.glyph_index('W').unwrap();
    
    let bitmap = face.glyph_raster_image(glyph_index, 16).unwrap();
}

pub fn extend_bitmap(bitmap: Vec<u8>, metrics: Metrics, size: usize) -> Vec<u8> {
    // extend the bitmap on every axis by size filling the new pixels with 0
    let mut new_bitmap = vec![0; (metrics.width + size * 2) as usize * (metrics.height + size * 2) as usize];
    for y in 0..metrics.height as usize {
        for x in 0..metrics.width as usize {
            let index = y * metrics.width as usize + x;
            let new_index = (y + size) * (metrics.width + size * 2) as usize + (x + size);
            new_bitmap[new_index] = bitmap[index];
        }
    }
    return new_bitmap;
}

pub fn sdf_render_test() {
    let fontfile = bolt::util::find_asset("fonts/Montserrat/Montserrat-VariableFont_wght.ttf");
    if fontfile.is_none() {
        panic!("Could not find font file");
    }
    let path = fontfile.unwrap();
    
    // load binary data from path to font file
    let font_data = std::fs::read(path).unwrap();
    
    let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default()).unwrap();
    // Rasterize and get the layout metrics for the letter 'g' at 17px.
    let res = 64.0;
    
    let (metrics, bitmap) = font.rasterize('g', res);
    
    let buffer: usize = (res / 8.0) as usize;
    let bitmap = extend_bitmap(bitmap, metrics, buffer);
    
    let sdfbm = BitmapGlyph::new(bitmap, metrics.width, metrics.height, buffer).unwrap();
    let sdf: Vec<f64> = sdfbm.render_sdf(8);
    
    println!("sdf: {:?}", sdf);
    println!("len: {:?}", sdf.len());
    println!("width: {:?}", metrics.width + (buffer * 2));
    println!("height: {:?}", metrics.height + (buffer * 2));
    println!("width * height: {:?}", (metrics.width + (buffer * 2)) * (metrics.height + (buffer * 2)));

    // create DynamicImage from sdf
    let mut image = DynamicImage::new_rgba8((metrics.width + buffer * 2) as u32, (metrics.height + buffer * 2) as u32);
    for (i, pixel) in sdf.iter().enumerate() {
        let x = i % (metrics.width + buffer * 2) as usize;
        let y = i / (metrics.height + buffer * 2) as usize;
        let color = (255.0 * pixel) as u8;
        image.put_pixel(x as u32, y as u32, image::Rgba([color, color, color, 255]));
    }
    
    image.into_rgba8().save("mysdf.png").unwrap();
}

fn prepare_context(settings: AppSettings, event_loop: &EventLoop<()>) -> (Window, Arc<SharedContext>, RendererSettings) {
    let mut window = Window::new(
        settings.resolution[0],
        settings.resolution[1],
        settings.name.clone(),
        &event_loop,
    );

    // create the context
    let shared_context = Arc::new(SharedContext::new(&mut window, &settings.render.clone()));
    let renderer = AppRenderer::new(&mut window, shared_context.clone(), settings.render.clone());

    (window, shared_context, settings.render.clone())
}

pub fn harfbuzz_test(unicode_string: &str) {
    let fontfile = bolt::util::find_asset("fonts/Montserrat/Montserrat-VariableFont_wght.ttf");
    if fontfile.is_none() {
        panic!("Could not find font file");
    }
    let path = fontfile.unwrap();
    let font_data = std::fs::read(path).unwrap();
    
    // load binary data from path to font file
    let face = ttf_parser::Face::parse(&font_data, 0).unwrap();
    use bolt::ui::text::parsing::OutlineBuilder;
    let mut builder = OutlineBuilder(Vec::new());

    let glyph_id = face.glyph_index('A').unwrap();

    face.outline_glyph(glyph_id, &mut builder);
    let mut outline = builder.0;
    println!("outline: {:?}", outline);
}

fn test_atlas() {
    let (mut window, shared_context, render_settings) = prepare_context(AppSettings::default(), &EventLoop::new());
    let renderer = bolt::AppRenderer::new(&mut window, shared_context.clone(), render_settings);
    let atlas = ftt_to_atlas(renderer.context.clone(), &PathBuf::from("fonts/Montserrat/Montserrat-VariableFont_wght.ttf"));
}


pub fn main() {
    sdf_render_test();
    harfbuzz_test("Hello World");
}
