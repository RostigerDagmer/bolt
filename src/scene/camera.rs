use glam::*;
use winit::event::{WindowEvent, VirtualKeyCode, ElementState};

pub enum CameraMode {
    Examine,
    Fly,
    Walk,
    Trackball,
    Spherical,
}
enum Actions {
    None,
    Orbit,
    Dolly,
    Pan,
    LookAround,
}
#[derive(Default, Debug, Clone, Copy)]
pub struct CameraInput {
    pub lmb: bool,
    pub mmb: bool,
    pub rmb: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub q: bool,
    pub e: bool,
    pub r: bool,
}

impl CameraInput {
    pub fn is_mouse_down(&self) -> bool {
        self.lmb || self.mmb || self.rmb
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Camera {
    input: CameraInput,
    position: Vec3,
    center: Vec3,
    up: Vec3,
    vfov: f32,
    min_vfov: f32,
    max_vfov: f32,
    z_near: f32,
    z_far: f32,
    view_matrix: Mat4,
    persp_matrix: Mat4,
    mouse_pos: Vec2,
    window_size: Vec2,
    speed: f32,
}

fn is_zero(value: f32) -> bool {
    value.abs() < f32::EPSILON
}

impl Camera {
    pub fn new(window_size: Vec2) -> Self {
        let mut camera = Camera {
            input: CameraInput::default(),
            position: Vec3::splat(10.0),
            center: Vec3::ZERO,
            up: Vec3::Y,
            vfov: 35.0,
            min_vfov: 1.0,
            max_vfov: 160.0,
            z_near: 0.1,
            z_far: 1000.0,
            view_matrix: Mat4::IDENTITY,
            persp_matrix: Mat4::IDENTITY,
            mouse_pos: Vec2::ZERO,
            window_size,
            speed: 30.0,
        };
        camera.update_persp();
        camera
    }

    pub fn from_view(view:Mat4, yfov:f32, z_near:f32, z_far:f32) -> Self {
        let view_inverse = view.inverse();
        let position = view_inverse * vec4(0.0,0.0,0.0,1.0);
        let up = view_inverse * vec4(0.0,1.0,0.0,0.0);
        let center = position + view_inverse * vec4(0.0,0.0,-4.0,0.0);
        
        let camera = Camera {
            input: CameraInput::default(),
            position: position.xyz(),
            center: center.xyz(),
            up: up.xyz(),
            vfov: yfov,
            min_vfov: 1.0,
            max_vfov: 160.0,
            z_near,
            z_far,
            view_matrix: view,
            persp_matrix: Mat4::IDENTITY,
            mouse_pos: Vec2::ZERO,
            window_size: vec2(1920.0, 1080.0),
            speed: 30.0,
        };
        camera
    }
}

impl Camera {
    fn update_view(&mut self) {
        self.view_matrix = Mat4::look_at_lh(self.position, self.center, -self.up);
    }

    fn update_persp(&mut self) {
        let aspect = self.window_size.x / self.window_size.y;
        // self.persp_matrix =
            // Mat4::perspective_lh(self.vfov.to_radians(), aspect, self.z_near, self.z_far);
        let h = 1.0 / (self.vfov.to_radians() * 0.5).tan();
        let w = h / aspect;
        let a = -self.z_near / (self.z_far - self.z_near);
        let b = (self.z_near * self.z_far) / (self.z_far - self.z_near);

        let r0 = glam::vec4(w, 0.0, 0.0, 0.0);
        let r1 = glam::vec4(0.0, -h, 0.0, 0.0);
        let r2 = glam::vec4(0.0, 0.0, a, 1.0);
        let r3 = glam::vec4(0.0, 0.0, b, 0.0);

        self.persp_matrix = glam::mat4(r0, r1, r2, r3);
    }

    pub fn look_at(&mut self, eye: Vec3, center: Vec3, up: Vec3) {
        self.position = eye;
        self.center = center;
        self.up = up;
        self.update_view();
    }

    pub fn set_window_size(&mut self, window_size: Vec2) {
        self.window_size = window_size;
        self.update_persp();
    }

    pub fn set_mouse_pos(&mut self, x: f32, y: f32) {
        self.mouse_pos = vec2(x, y);
    }

    pub fn set_vfov(&mut self, vfov: f32) {
        self.vfov = vfov;
        self.update_persp();
    }

    pub fn change_vfov(&mut self, delta: f32) {
        self.vfov += delta;
        self.vfov = self.vfov.max(self.min_vfov).min(self.max_vfov);
        self.update_persp();
    }

    pub fn key_move(&mut self, input: &CameraInput, speed: f32) -> bool {
        let move_direction = if input.w {
            Vec3::Z
        } else if input.s {
            -Vec3::Z
        } else {
            Vec3::ZERO
        } + if input.a {
            Vec3::X
        } else if input.d {
            -Vec3::X
        } else {
            Vec3::ZERO
        } + if input.q {
            Vec3::Y
        } else if input.e {
            -Vec3::Y
        } else {
            Vec3::ZERO
        };
        if !is_zero(move_direction.x) || !is_zero(move_direction.y) || !is_zero(move_direction.z) {
            let z = (self.center - self.position).normalize();
            let x = self.up.cross(z).normalize();
            let y = z.cross(x).normalize();
            let direction = move_direction.x * x + move_direction.y * y + move_direction.z * z;
            self.position += direction * speed;
            self.center += direction * speed;
            self.update_view();
            return true;
        }
        false
    }

    pub fn mouse_move(&mut self, x: f32, y: f32, input: &CameraInput) -> bool {
        let mut moved = false;
        let mut action = Actions::None;

            // if ((input.ctrl) && (input.shift)) || input.alt {
            //     action = Actions::Orbit;
            // } else if input.shift {
            //     action = Actions::Dolly;
            // } else if input.ctrl {
            //     action = Actions::Pan;
            // } else {
            //     action = Actions::LookAround;
            // }
            
        if input.mmb {
            if input.shift {
                action = Actions::Orbit;
            } else {
                action = Actions::Pan;
            }
            
        } else if input.rmb {
            if input.shift {
                action = Actions::Dolly;
            } else {
                action = Actions::LookAround;
            }
        }

        let dx = (x - self.mouse_pos.x) / self.window_size.x;
        let dy = (y - self.mouse_pos.y) / self.window_size.y;
        match action {
            Actions::None => {}
            Actions::Orbit => {
                self.orbit(dx, -dy);
                moved = true;
            }
            Actions::Dolly => {
                self.dolly(0.0, dy);
                moved = true;
            }
            Actions::Pan => {
                self.pan(dx, -dy);
                moved = true;
            }
            Actions::LookAround => {
                self.look_around(dx, -dy);
                moved = true;
            }
        }
        if moved {
            self.update_view();
        }
        self.mouse_pos = vec2(x, y);

        moved
    }

    pub fn mouse_wheel(&mut self, value: i32) {
        let fval = value as f32;
        let dx = fval * fval.abs() / self.window_size.x;
        self.dolly(0.0, -dx * self.speed);
        self.update_view();
    }

    fn pan(&mut self, dx: f32, dy: f32) {
        let z = self.position - self.center;
        let length = z.length() / 0.785; // 45 degrees
        let z = z.normalize();
        let mut x = self.up.cross(z).normalize();
        let mut y = z.cross(x).normalize();

        x *= -dx * length;
        y *= dy * length;

        self.position += x + y;
        self.center += x + y;
    }

    fn orbit(&mut self, mut dx: f32, mut dy: f32) {
        if is_zero(dx) && is_zero(dy) {
            return;
        }

        // Full width will do a full turn
        dx *= std::f32::consts::TAU;
        dy *= std::f32::consts::TAU;

        // Get the camera
        let origin = self.center;
        let position = self.position;

        // Get the length of sight
        let mut center_to_eye = position - origin;
        let radius = center_to_eye.length();
        center_to_eye = center_to_eye.normalize();

        // Find the rotation around the UP axis (Y)
        let axe_z = center_to_eye;
        let rot_y = Mat4::from_axis_angle(self.up, -dx);

        // Apply the (Y) rotation to the eye-center vector
        let mut tmp = rot_y.mul_vec4(vec4(center_to_eye.x, center_to_eye.y, center_to_eye.z, 0.0));
        center_to_eye = vec3(tmp.x, tmp.y, tmp.z);

        // Find the rotation around the X vector: cross between eye-center and up (X)
        let axe_x = self.up.cross(axe_z).normalize();
        let rot_x = Mat4::from_axis_angle(axe_x, -dy);

        // Apply the (X) rotation to the eye-center vector
        tmp = rot_x.mul_vec4(vec4(center_to_eye.x, center_to_eye.y, center_to_eye.z, 0.0));
        let vect_rot = vec3(tmp.x, tmp.y, tmp.z);
        if vect_rot.x.signum() == center_to_eye.x.signum() {
            center_to_eye = vect_rot;
        }

        // Make the vector as long as it was originally
        center_to_eye *= radius;

        // Finding the new position
        self.position = origin + center_to_eye;
    }

    fn dolly(&mut self, dx: f32, dy: f32) {
        let mut z = self.center - self.position;
        let mut length = z.length();
        if is_zero(length) {
            return;
        }

        // Use the larger movement.
        let dd = if dx.abs() > dy.abs() { dx } else { -dy };
        let mut factor = self.speed * dd / length;

        // Adjust speed based on distance.
        length /= 10.0;
        length = length.max(0.001);
        factor *= length;

        // Don't move to or through the point of interest.
        if factor >= 1.0 {
            return;
        }

        z *= factor;
        self.position += z;
    }

    fn look_around(&mut self, dx: f32, dy: f32) {
        let mut z = self.position - self.center;
        let length = z.length();
        z = z.normalize();
        let mut x = self.up.cross(z).normalize();
        let mut y = z.cross(x).normalize();

        x *= -dx * length;
        y *= dy * length;

        self.position += x + y;
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    pub fn perspective_matrix(&self) -> Mat4 {
        self.persp_matrix
    }
}

pub struct CameraManip {
    pub input: CameraInput,
    pub camera: Camera,
    pub mode: CameraMode,
    pub speed: f32,
}

impl Default for CameraManip {
    fn default() -> Self {
        Self {
            input: CameraInput::default(),
            camera: Camera::default(),
            mode: CameraMode::Spherical,
            speed: 1.0,
        }
    }
}

impl CameraManip {
    pub fn update(&mut self, window_event: &WindowEvent) -> bool {
        let mut moved = false;
        match window_event {
            WindowEvent::Resized(winit::dpi::PhysicalSize { width, height }) => {
                self.camera
                    .set_window_size(vec2(*width as f32, *height as f32));
            }
            _ => {
                match self.mode {
                    CameraMode::Spherical => {
                        moved = self.update_spherical(window_event);
                    }
                    CameraMode::Fly => {
                        moved = self.update_fly(window_event);
                    }
                    _ => {}
                };
            }
        }
        moved
    }

    pub fn update_spherical(&mut self, window_event: &WindowEvent) -> bool {
        let mut moved = false;
        match window_event {
            WindowEvent::ModifiersChanged(m) => {
                self.input.alt = m.alt();
                self.input.ctrl = m.ctrl() || m.logo();
                self.input.shift = m.shift();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = vec2(position.x as f32, position.y as f32);
                if self.input.is_mouse_down() {
                    moved = self.camera.mouse_move(pos.x, pos.y, &self.input);
                } else {
                    self.camera.set_mouse_pos(pos.x, pos.y);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::PixelDelta(_) => {
                        //camera.mouse_wheel(d.x.max(d.y) as i32);
                    }
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        self.camera.mouse_wheel(-(*y) as i32);
                        moved = true;
                    }
                };
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let is_down = match state {
                    winit::event::ElementState::Pressed => true,
                    winit::event::ElementState::Released => false,
                };
                match button {
                    winit::event::MouseButton::Left => {
                        self.input.lmb = is_down;
                    }
                    winit::event::MouseButton::Right => {
                        self.input.rmb = is_down;
                    }
                    winit::event::MouseButton::Middle => {
                        self.input.mmb = is_down;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        moved
    }

    pub fn update_fly(&mut self, window_event: &WindowEvent) -> bool {
        let mut moved = false;
        match window_event {
            // handle w a s d key inputs
            WindowEvent::ModifiersChanged(m) => {
                self.input.alt = m.alt();
                self.input.ctrl = m.ctrl() || m.logo();
                self.input.shift = m.shift();
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let is_down = match state {
                    winit::event::ElementState::Pressed => true,
                    winit::event::ElementState::Released => false,
                };
                match button {
                    winit::event::MouseButton::Left => {
                        self.input.lmb = is_down;
                    }
                    winit::event::MouseButton::Right => {
                        self.input.rmb = is_down;
                    }
                    winit::event::MouseButton::Middle => {
                        self.input.mmb = is_down;
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    match keycode {
                        VirtualKeyCode::W => {
                            self.input.w = input.state == ElementState::Pressed;
                        }
                        VirtualKeyCode::A => {
                            self.input.a = input.state == ElementState::Pressed;
                        }
                        VirtualKeyCode::S => {
                            self.input.s = input.state == ElementState::Pressed;
                        }
                        VirtualKeyCode::D => {
                            self.input.d = input.state == ElementState::Pressed;
                        }
                        VirtualKeyCode::Q => {
                            self.input.q = input.state == ElementState::Pressed;
                        }
                        VirtualKeyCode::E => {
                            self.input.e = input.state == ElementState::Pressed;
                        }
                        _ => {}
                    }
                }
                moved = self.camera.key_move(&self.input, self.speed);
            }

            WindowEvent::CursorMoved { position, .. } => {
                let pos = vec2(position.x as f32, position.y as f32);
                if self.input.is_mouse_down() {
                    moved = self.camera.mouse_move(pos.x, pos.y, &self.input);
                } else {
                    self.camera.set_mouse_pos(pos.x, pos.y);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::PixelDelta(_) => {
                        //camera.mouse_wheel(d.x.max(d.y) as i32);
                    }
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        self.camera.change_vfov(*y);
                        //self.camera.mouse_wheel(*y as i32);
                        moved = true;
                    }
                };
            }
            _ => {}
        }
        moved
    }
}
