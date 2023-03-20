pub mod events;

use std::sync::mpsc::Receiver;

use aeonetica_engine::log;
use crate::{renderer::{context::Context, buffer::*, shader::*, util, Camera}};

extern crate glfw;
extern crate gl;
use glfw::{*, Window as GlfwWindow, Context as GlfwContext};

use super::{Renderer, vertex_array::VertexArray};

pub(crate) struct Window {
    glfw_handle: Glfw,
    glfw_window: GlfwWindow,
    event_receiver: Receiver<(f64, WindowEvent)>,
    renderer: Renderer,
    test_vao: VertexArray,
    test_camera: Camera,
    context: Context
}

impl Window {
    const DEFAULT_WINDOW_WIDTH: u32 = 1280;
    const DEFAULT_WINDOW_HEIGHT: u32 = 720;
    const DEFAULT_WINDOW_TITLE: &'static str = "Aeonetica Game Engine";

    pub(crate) fn new(full_screen: bool, context: Context) -> Self {
        match glfw::init(glfw::FAIL_ON_ERRORS) {
            Ok(mut glfw) => {
                let (mut window, events) = glfw.with_primary_monitor(|glfw, monitor| {
                    glfw.create_window(
                    Self::DEFAULT_WINDOW_WIDTH,
                    Self::DEFAULT_WINDOW_HEIGHT,
                    Self::DEFAULT_WINDOW_TITLE,
                    if full_screen {
                        monitor.map_or(WindowMode::Windowed, |m| WindowMode::FullScreen(m))
                    } else {
                        WindowMode::Windowed
                    }
                )}).expect("Error creating GLFW window!");

                window.make_current();
                window.set_key_polling(true);

                gl::load_with(|s| glfw.get_proc_address_raw(s));
                gl::Viewport::load_with(|s| glfw.get_proc_address_raw(s));

                log!(r#"
==== OpenGL info ====
  -> Vendor: {}
  -> Renderer: {}
  -> Version: {}"#, 
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8).to_str().unwrap() },
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8).to_str().unwrap() },
                    unsafe { std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8).to_str().unwrap() }
                );

                let mut test_vao = VertexArray::new().expect("Error creating vertex array");
                test_vao.bind();

                type Vertex = [f32; 3];
                type Color = [f32; 3];

                const VERTICES: [(Vertex, Color); 4] = [
                    ([-0.5, -0.5, 0.0], [1.0, 0.0, 0.0]),
                    ([0.5,  -0.5, 0.0], [0.0, 1.0, 0.0]),
                    ([0.5,  0.5,  0.0], [0.0, 0.0, 1.0]),
                    ([-0.5, 0.5,  0.0], [0.0, 0.0, 0.0])
                ];

                let layout = BufferLayout::new(vec![
                    BufferElement::new(ShaderDataType::Float3), // position
                    BufferElement::new(ShaderDataType::Float3), // color
                ]);
                
                let vertex_buffer = Buffer::new(BufferType::Array, util::to_raw_byte_slice!(VERTICES), Some(layout))
                    .expect("Error creating Vertex Buffer");
                test_vao.add_vertex_buffer(vertex_buffer);
                
                const INDICES: [u32; 6] = [ 0, 1, 2, 2, 3, 0 ];
                let index_buffer = Buffer::new(BufferType::ElementArray, util::to_raw_byte_slice!(INDICES), None)
                    .expect("Error creating Index Buffer");
                test_vao.set_index_buffer(index_buffer);

                let renderer = Renderer::new();

                let aspect_ratio = 16.0 / 9.0;
                let zoom = 1.0;
                let camera = Camera::new(-aspect_ratio * zoom, aspect_ratio * zoom, -zoom, zoom, -1.0, 1.0);

                Self {
                    glfw_handle: glfw,
                    glfw_window: window,
                    event_receiver: events,
                    renderer,
                    test_vao,
                    test_camera: camera,
                    context
                }
            },
            Err(err) => panic!("Error creating window: {err}!") 
        }
    }

    pub(crate) fn poll_events(&mut self) {
        self.glfw_handle.poll_events();
        for (_, event) in flush_messages(&self.event_receiver) {
            let event = events::Event::from_glfw(event);

            if let events::EventType::WindowClose() = event.typ() {
                self.glfw_window.set_should_close(true);
            }

            self.context.on_event(event);
        }
    }

    pub(crate) fn render(&mut self) {
        unsafe {
            gl::Viewport(0, 0, self.glfw_window.get_size().0, self.glfw_window.get_size().1);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
            gl::ClearColor(0.1, 0.1, 0.2, 0.0);
        }
        
        let aspect_ratio = self.glfw_window.get_size().0 as f32 / self.glfw_window.get_size().1 as f32;
        let zoom = 1.0;
        self.test_camera.set_projection(-aspect_ratio * zoom, aspect_ratio * zoom, -zoom, zoom, -1.0, 1.0);

        // render here
        self.renderer.begin_scene(&self.test_camera);
        self.renderer.draw_vertex_array(&self.test_vao);
        self.renderer.end_scene();

        self.glfw_window.swap_buffers();
    }

    pub(crate) fn should_close(&self) -> bool {
        self.glfw_window.should_close()
    }
}
