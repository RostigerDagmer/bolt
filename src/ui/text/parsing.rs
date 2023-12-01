use glam::Vec2;

pub struct OutlineBuilder(pub Vec<Contour>);

#[derive(Debug)]
pub enum Contour {
    MoveTo(Vec2),
    LineTo(Vec2),
    QuadTo([Vec2; 2]),
    CurveTo([Vec2; 3]),
    Close,
}

impl ttf_parser::OutlineBuilder for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Contour::MoveTo(Vec2::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Contour::LineTo(Vec2::new(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(Contour::QuadTo([Vec2::new(x1, y1), Vec2::new(x, y)]));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push(Contour::CurveTo([Vec2::new(x1, y1), Vec2::new(x2, y2), Vec2::new(x, y)])); 
    }

    fn close(&mut self) {
        self.0.push(Contour::Close);
    }
}