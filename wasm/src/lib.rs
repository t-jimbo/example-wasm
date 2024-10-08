extern crate nalgebra_glm as glm;

use std::{
    cell::RefCell,
    f32::consts,
    rc::Rc,
};
use wasm_bindgen::{
    prelude::*,
    JsCast,
};
use web_sys::{
    WebGl2RenderingContext as GL,
    *,
};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let document = window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas")
        .ok_or("canvas not found")?
        .dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(768);
    canvas.set_height(768);

    let gl = canvas.get_context("webgl2")?
        .ok_or("Failed to get WebGl2RenderingContext")?
        .dyn_into::<GL>()?;

    let program = create_program(&gl)?;
    gl.use_program(Some(&program));

    let vertices = get_vertices();
    let colors = get_colors();
    let indices = get_indices();

    let vbo_data: &[&[f32]] = &[&vertices, &colors];
    let locations = &[0, 1];
    let vertex_count = vertices.len() as i32 / 3;

    let vao = create_vao(&gl, vbo_data, locations, &indices, vertex_count)?;
    gl.bind_vertex_array(Some(&vao));

    let mvp_location = gl.get_uniform_location(&program, "mvpMatrix").ok_or("Failed to get uniform location")?;

    gl.enable(GL::DEPTH_TEST);
    gl.depth_func(GL::LEQUAL);
    gl.enable(GL::CULL_FACE);

    let index_count = indices.len() as i32;
    let mut frame_count = 0;

    let closure = Rc::new(RefCell::new(None));
    let clone = closure.clone();
    *clone.borrow_mut() = Some(Closure::<dyn FnMut() -> Result<i32, JsValue>>::new(move || {
        frame_count += 1;
        send_mvp_matrix(&gl, &mvp_location, &canvas, frame_count);
        draw(&gl, index_count);
        request_animation_frame(closure.borrow().as_ref().unwrap())
    }));
    request_animation_frame(clone.borrow().as_ref().unwrap())?;

    Ok(())
}

fn create_program(gl: &GL) -> Result<WebGlProgram, String> {
    let vertex_shader = create_shader(&gl, GL::VERTEX_SHADER, include_str!("shader/vertex.glsl"))?;
    let fragment_shader = create_shader(&gl, GL::FRAGMENT_SHADER, include_str!("shader/fragment.glsl"))?;

    let program = gl.create_program().ok_or("Failed to create program object")?;
    gl.attach_shader(&program, &vertex_shader);
    gl.attach_shader(&program, &fragment_shader);
    gl.link_program(&program);

    if gl.get_program_parameter(&program, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(program)
    } else {
        let log = gl.get_program_info_log(&program).unwrap_or(String::from("Failed to link program"));
        gl.delete_program(Some(&program));
        Err(log)
    }
}

fn create_shader(gl: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(shader_type).ok_or("Failed to create shader object")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl.get_shader_parameter(&shader, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or(String::from("Failed to compile shader"));
        gl.delete_shader(Some(&shader));
        Err(log)
    }
}

fn get_vertices() -> Vec<f32> {
    vec![
        // 前面
        -0.5, -0.5, 0.5,
        0.5, -0.5, 0.5,
        0.5, 0.5, 0.5,
        -0.5, 0.5, 0.5,
        // 背面
        -0.5, -0.5, -0.5,
        -0.5, 0.5, -0.5,
        0.5, 0.5, -0.5,
        0.5, -0.5, -0.5,
        // 上面
        -0.5, 0.5, -0.5,
        -0.5, 0.5, 0.5,
        0.5, 0.5, 0.5,
        0.5, 0.5, -0.5,
        // 下面
        -0.5, -0.5, -0.5,
        0.5, -0.5, -0.5,
        0.5, -0.5, 0.5,
        -0.5, -0.5, 0.5,
        // 右面
        0.5, -0.5, -0.5,
        0.5, 0.5, -0.5,
        0.5, 0.5, 0.5,
        0.5, -0.5, 0.5,
        // 左面
        -0.5, -0.5, -0.5,
        -0.5, -0.5, 0.5,
        -0.5, 0.5, 0.5,
        -0.5, 0.5, -0.5,
    ]
}

fn get_colors() -> Vec<f32> {
    [
        1.0, 0.0, 0.0, 1.0,
        0.0, 1.0, 0.0, 1.0,
        0.0, 0.0, 1.0, 1.0,
        1.0, 1.0, 0.0, 1.0,
    ].repeat(6)
}

fn get_indices() -> Vec<u16> {
    let vertex_indices = [
        0, 1, 2,
        0, 2, 3,
    ];
    [vertex_indices; 6].iter().enumerate()
        .flat_map(|(i, v)| v.iter().map(move |u| u + 4 * i as u16))
        .collect::<Vec<_>>()
}

fn create_vao(
    gl: &GL,
    vbo_data: &[&[f32]],
    locations: &[u32],
    ibo_data: &[u16],
    vertex_count: i32,
) -> Result<WebGlVertexArrayObject, String> {
    let vao = gl.create_vertex_array().ok_or("Failed to create vertex array object")?;
    gl.bind_vertex_array(Some(&vao));

    for i in 0..vbo_data.len() {
        let vbo = gl.create_buffer().ok_or("Failed to create buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        unsafe {
            let view = js_sys::Float32Array::view(&vbo_data[i]);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }
        gl.enable_vertex_attrib_array(locations[i]);
        let size = vbo_data[i].len() as i32 / vertex_count;
        gl.vertex_attrib_pointer_with_i32(locations[i], size, GL::FLOAT, false, 0, 0);
    }

    let ibo = gl.create_buffer().ok_or("Failed to create buffer")?;
    gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
    unsafe {
        let view = js_sys::Uint16Array::view(ibo_data);
        gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &view, GL::STATIC_DRAW);
    }

    gl.bind_vertex_array(None);

    Ok(vao)
}

fn send_mvp_matrix(
    gl: &GL,
    location: &WebGlUniformLocation,
    canvas: &HtmlCanvasElement,
    frame_count: i32,
) {
    let radians = (frame_count % 360) as f32 * consts::PI / 180.0;
    let axis = glm::Vec3::new(1.0, 1.0, 1.0);
    let model_matrix = glm::rotate(&glm::Mat4::identity(), radians, &axis);

    let eye = glm::Vec3::new(0.0, 0.0, 3.0);
    let center = glm::Vec3::new(0.0, 0.0, 0.0);
    let up = glm::Vec3::new(0.0, 1.0, 0.0);
    let view_matrix = glm::look_at(&eye, &center, &up);

    let aspect = canvas.width() as f32 / canvas.height() as f32;
    let fovy = 45.0 * consts::PI / 180.0;
    let near = 0.1;
    let far = 10.0;
    let projection_matrix = glm::perspective(aspect, fovy, near, far);

    let mvp_matrix = projection_matrix * view_matrix * model_matrix;
    let mvp_arrays: [[f32; 4]; 4] = mvp_matrix.into();
    let mvp_matrices = mvp_arrays.iter().flat_map(|a| *a).collect::<Vec<_>>();

    gl.uniform_matrix4fv_with_f32_array_and_src_offset_and_src_length(Some(location), false, &mvp_matrices, 0, 0);
}

fn draw(gl: &GL, index_count: i32) {
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear_depth(1.0);
    gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);

    gl.draw_elements_with_i32(GL::TRIANGLES, index_count, GL::UNSIGNED_SHORT, 0);
    gl.flush();
}

fn request_animation_frame(
    closure: &Closure<dyn FnMut() -> Result<i32, JsValue>>
) -> Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    window.request_animation_frame(closure.as_ref().unchecked_ref())
}
