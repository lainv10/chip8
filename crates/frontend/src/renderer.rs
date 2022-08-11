use eframe::glow;
use glow::*;

/// A renderer for displaying the graphics
/// buffer of the `Chip8` using an OpenGL renderer.
pub struct Renderer {
    program: ShaderProgram,
    vbo: Buffer,
    vao: VertexArray,
    texture: Texture,
}

impl Renderer {
    /// Create a new renderer with a [`glow::Context`].
    /// This will run OpenGL initialization code with the given context.
    /// All subsequent calls to this `Renderer` should pass in the same context.
    pub fn new(gl: &glow::Context) -> Self {
        let (vbo, vao) = unsafe { Self::create_quad(gl) };
        let texture = unsafe { gl.create_texture().unwrap() };
        let program = Self::create_shader_program(gl);
        unsafe { gl.clear_color(0.0, 0.0, 0.0, 1.0) };
        Self {
            program,
            vbo,
            vao,
            texture,
        }
    }

    /// Load shader sources and create a [`ShaderProgram`].
    fn create_shader_program(gl: &glow::Context) -> ShaderProgram {
        let vertex_shader_source = include_str!("./vertex.glsl");
        let fragment_shader_source = include_str!("./fragment.glsl");
        ShaderProgram::new(gl, vertex_shader_source, fragment_shader_source)
    }

    /// Create a quad to render a texture to.
    unsafe fn create_quad(gl: &glow::Context) -> (Buffer, VertexArray) {
        // (pos.x, pos.y, pos.z, tex.s, tex.t)
        let triangle_vertices = [
            1f32, 1.0, 0.0, 1.0, 0.0, // 1
            1.0, -1.0, 0.0, 1.0, 1.0, // 2
            -1.0, -1.0, 0.0, 0.0, 1.0, // 3
            -1.0, 1.0, 0.0, 0.0, 0.0, // 4
        ];
        // cast to byte slice
        let triangle_vertices_u8 = core::slice::from_raw_parts(
            triangle_vertices.as_ptr() as *const u8, // byte pointer
            triangle_vertices.len() * core::mem::size_of::<f32>(), // size of all vertex data
        );

        let triangle_indices = [0u32, 1, 3, 1, 2, 3];
        let triangle_indices_u8 = core::slice::from_raw_parts(
            triangle_indices.as_ptr() as *const u8,
            triangle_indices.len() * core::mem::size_of::<f32>(),
        );

        let vao = gl.create_vertex_array().unwrap();
        let vbo = gl.create_buffer().unwrap();
        let ebo = gl.create_buffer().unwrap();
        gl.bind_vertex_array(Some(vao));

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, triangle_vertices_u8, glow::STATIC_DRAW);

        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            triangle_indices_u8,
            glow::STATIC_DRAW,
        );

        let stride = 5 * i32::try_from(std::mem::size_of::<f32>()).unwrap();
        gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
        gl.enable_vertex_attrib_array(0);

        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 12);
        gl.enable_vertex_attrib_array(1);

        (vbo, vao)
    }

    /// Load the given RGB buffer as a  texture into the given OpenGL context.
    unsafe fn load_texture(&mut self, gl: &glow::Context, buffer: &[u8]) {
        let texture = gl.create_texture().unwrap();
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            chip8::graphics::WIDTH as i32,
            chip8::graphics::HEIGHT as i32,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            Some(buffer),
        );

        gl.delete_texture(self.texture);
        self.texture = texture;
    }

    /// Render the given buffer of RGB data onto a texture.
    pub fn render(&mut self, gl: &glow::Context, buffer: &[u8]) {
        unsafe {
            self.load_texture(gl, buffer);
            self.program.use_program(gl);
            gl.bind_vertex_array(Some(self.vao));

            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);
        }
    }

    /// Clean up state from the GL context.
    pub fn clean_up(&mut self, gl: &glow::Context) {
        self.program.delete(gl);
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
        }
    }
}

/// An OpenGL shader program.
pub struct ShaderProgram {
    program: glow::Program,
}

impl ShaderProgram {
    /// Create a new shader program with the given vertex and fragment shader sources.
    pub fn new(gl: &glow::Context, vertex_src: &str, fragment_src: &str) -> Self {
        unsafe {
            let program = gl
                .create_program()
                .expect("failed to create shader program");

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_src),
                (glow::FRAGMENT_SHADER, fragment_src),
            ];
            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in shader_sources.iter() {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("failed to create shader");
                gl.shader_source(shader, shader_source);
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    let shader_type_string = match *shader_type {
                        glow::VERTEX_SHADER => "vertex",
                        glow::FRAGMENT_SHADER => "fragment",
                        _ => "",
                    };
                    log::error!(
                        "{} shader failed to compile: {}",
                        shader_type_string,
                        gl.get_shader_info_log(shader)
                    );
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                log::error!(
                    "shader program linking failed: {}",
                    gl.get_program_info_log(program)
                );
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            Self { program }
        }
    }

    /// Use this shader program.
    pub fn use_program(&self, gl: &glow::Context) {
        unsafe { gl.use_program(Some(self.program)) }
        self.set_uniform_i32(gl, "tex", 0);
    }

    /// Set an `i32` uniform.
    pub fn set_uniform_i32(&self, gl: &glow::Context, name: &str, value: i32) {
        unsafe {
            let uniform_location = gl.get_uniform_location(self.program, name);
            gl.uniform_1_i32(uniform_location.as_ref(), value);
        }
    }

    /// Delete this shader program. The program should not be used again after a call to this.
    pub fn delete(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
        }
    }
}
