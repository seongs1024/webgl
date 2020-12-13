mod utils;

use std::{
    cell::RefCell,
    rc::Rc,
    f32::consts::PI,
};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    WebGlProgram, WebGlRenderingContext, WebGlShader,
    WebGlUniformLocation, WebGlBuffer,
    EventTarget, MouseEvent,
};
use js_sys::WebAssembly;

const AMORTIZATION: f32 = 0.95;

#[derive(Debug, Clone)]
struct ProgramInfo(
    WebGlProgram,
    (u32, u32),
    (
        Result<WebGlUniformLocation, String>,
        Result<WebGlUniformLocation, String>,
    ),
);

#[wasm_bindgen()]
pub fn start(canvas_id: &str) -> Result<(), JsValue> {
    // Create a canvas
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;

    // Compile the shader program
    let vert_shader = compile_shader(
        &context,
        WebGlRenderingContext::VERTEX_SHADER,
        r#"
        attribute vec4 position;
        attribute vec4 color;

        uniform mat4 projection_matrix;
        uniform mat4 model_view_matrix;

        varying lowp vec4 vColor;

        void main() {
            gl_Position = projection_matrix * model_view_matrix * position;
            vColor = color;
        }
    "#,
    )?;
    let frag_shader = compile_shader(
        &context,
        WebGlRenderingContext::FRAGMENT_SHADER,
        r#"
        varying lowp vec4 vColor;

        void main() {
            gl_FragColor = vColor;
        }
    "#,
    )?;
    let program = link_program(&context, &vert_shader, &frag_shader)?;
    context.use_program(Some(&program));

    // Collect all the info needed to use the shader program.
    // Look up which attributes the program is using for "posision", "color",
    // and also look up uniform locations.
    let program_info = {
        let vertex_position = context
            .get_attrib_location(&program, "position") as u32;
        let vertex_color = context
            .get_attrib_location(&program, "color") as u32;
        let projection_matrix = context
            .get_uniform_location(&program, "projection_matrix")
            .ok_or_else(|| String::from("cannot get projection_matrix"));
        let model_view_matrix = context
            .get_uniform_location(&program, "model_view_matrix")
            .ok_or_else(|| String::from("cannot get model_view_matrix"));
        ProgramInfo(
            program,
            (vertex_position, vertex_color),
            (projection_matrix, model_view_matrix),
        )
    };

    // Call the routine that builds all the objects that will be drawed.
    let buffers: Buffers = init_buffers(&context)?;


    // Draw the scene repeatedly
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    let drag = Rc::new(RefCell::new(false));
    let theta = Rc::new(RefCell::new(0.0));
    let phi = Rc::new(RefCell::new(0.0));
    let dX = Rc::new(RefCell::new(0.0));
    let dY = Rc::new(RefCell::new(0.0));
    let canvas_width = Rc::new(RefCell::new(canvas.width() as f32));
    let canvas_height = Rc::new(RefCell::new(canvas.height() as f32));

    // get canvas as event target
    let event_target: EventTarget = canvas.into();

    // Add event listeners
    // MOUSEDOWN
    {
        let drag = drag.clone();
        let mousedown_cb = Closure::wrap(Box::new(move |_event: MouseEvent| {
            *drag.borrow_mut() = true;
        }) as Box<dyn FnMut(MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mousedown", mousedown_cb.as_ref().unchecked_ref())
            .unwrap();
        mousedown_cb.forget();
    }
    // MOUSEUP and MOUSEOUT
    {
        let drag = drag.clone();
        let mouseup_cb = Closure::wrap(Box::new(move |_event: MouseEvent| {
            *drag.borrow_mut() = false;
        }) as Box<dyn FnMut(MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mouseup", mouseup_cb.as_ref().unchecked_ref())
            .unwrap();
        event_target
            .add_event_listener_with_callback("mouseout", mouseup_cb.as_ref().unchecked_ref())
            .unwrap();
        mouseup_cb.forget();
    }
    // MOUSEMOVE
    {
        let theta = theta.clone();
        let phi = phi.clone();
        let canvas_width = canvas_width.clone();
        let canvas_height = canvas_height.clone();
        let dX = dX.clone();
        let dY = dY.clone();
        let drag = drag.clone();
        let mousemove_cb = Closure::wrap(Box::new(move |event: MouseEvent| {
            if *drag.borrow() {
                let cw = *canvas_width.borrow();
                let ch = *canvas_height.borrow();
                *dX.borrow_mut() = (event.movement_x() as f32) * 2.0 * PI / cw;
                *dY.borrow_mut() = (event.movement_y() as f32) * 2.0 * PI / ch;
                *theta.borrow_mut() += *dX.borrow();
                *phi.borrow_mut() += *dY.borrow();
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
        event_target
            .add_event_listener_with_callback("mousemove", mousemove_cb.as_ref().unchecked_ref())
            .unwrap();
        mousemove_cb.forget();
    }
    // RequestAnimationFrame
    {
        let dX = dX.clone();
        let dY = dY.clone();
        let drag = drag.clone();
        // Request animation frame
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |_d| {
            if !*drag.borrow() {
                *dX.borrow_mut() *= AMORTIZATION;
                *dY.borrow_mut() *= AMORTIZATION;
                *theta.borrow_mut() += *dX.borrow();
                *phi.borrow_mut() += *dY.borrow();
            }
            draw_scene(
                &context.clone(),
                program_info.clone(),
                buffers.clone(),
                *theta.borrow(),
                *phi.borrow(),
            )
            .unwrap();
            // Schedule ourself for another requestAnimationFrame callback.
            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<FnMut(f32)>));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }
    Ok(())

/*
    // Draw the scene repeatedly
    context.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);
    context.enable_vertex_attrib_array(0);

    context.clear_color(0.0, 0.0, 0.0, 1.0);
    context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

    context.draw_arrays(
        WebGlRenderingContext::TRIANGLES,
        0,
        (vertices.len() / 3) as i32,
    );
    Ok(())
*/
}

#[derive(Debug, Clone)]
struct Buffers(WebGlBuffer, WebGlBuffer, WebGlBuffer);

fn init_buffers(context: &WebGlRenderingContext) -> Result<Buffers, JsValue> {
    // Create a buffer for the cube's vertex positions.
    let position_buffer = context
        .create_buffer()
        .ok_or("failed to create position_buffer")?;
    // Select the position_buffer as the one to apply buffer operstions to from here out.
    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));

    // Create an array of positions for the cube.
    let positions: [f32; 72] = [
        // Front face
        -1.0, -1.0, 1.0, //
        1.0, -1.0, 1.0, //
        1.0, 1.0, 1.0, //
        -1.0, 1.0, 1.0, //
        // Back face
        -1.0, -1.0, -1.0, //
        -1.0, 1.0, -1.0, //
        1.0, 1.0, -1.0, //
        1.0, -1.0, -1.0, //
        // Top face
        -1.0, 1.0, -1.0, //
        -1.0, 1.0, 1.0, //
        1.0, 1.0, 1.0, //
        1.0, 1.0, -1.0, //
        // Bottom face
        -1.0, -1.0, -1.0, //
        1.0, -1.0, -1.0, //
        1.0, -1.0, 1.0, //
        -1.0, -1.0, 1.0, //
        // Right face
        1.0, -1.0, -1.0, //
        1.0, 1.0, -1.0, //
        1.0, 1.0, 1.0, //
        1.0, -1.0, 1.0, //
        // Left face
        -1.0, -1.0, -1.0, //
        -1.0, -1.0, 1.0, //
        -1.0, 1.0, 1.0, //
        -1.0, 1.0, -1.0, //
    ];
    let position_array = float_32_array!(positions);
    // Pass the list of positions into WebGL to build the shape.
    // Do this by creating a Float32Array from the Rust array,
    // then use it to fill the currect buffer.
    context.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &position_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    // Set up the color for the faces.
    // In this case, solid colors for each face is used.
    let color_buffer = context
        .create_buffer()
        .ok_or("failed to create color_buffer")?;
    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&color_buffer));

    let face_colors = [
        [1.0, 1.0, 1.0, 1.0], // Front face: white
        [1.0, 0.0, 0.0, 1.0], // Back face: red
        [0.0, 1.0, 0.0, 1.0], // Top face: green
        [0.0, 0.0, 1.0, 1.0], // Bottom face: blue
        [1.0, 1.0, 0.0, 1.0], // Right face: yellow
        [1.0, 0.0, 1.0, 1.0], // Left face: purple
    ];
    let color_array = {
        let color_vec: Vec<f32> = face_colors
            .iter()
            .map(|row| vec![row, row, row, row])
            .flatten()
            .flatten()
            .map(|x| *x)
            .collect();
        let mut color_arr: [f32; 96] = [0f32; 96];
        color_arr.copy_from_slice(color_vec.as_slice());
        float_32_array!(color_arr)
    };
   context.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ARRAY_BUFFER,
        &color_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    // Build the element array buffer; this specifies the indices
    // into the vertex arrays for each face's vertices.
    let index_buffer = context
        .create_buffer()
        .ok_or("failed to create index_buffer buffer")?;
    context.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&index_buffer),
    );
    // Define each face as two triangles, using the indies into the vertex array
    // to specify each triangle's position.
    let indices: [u16; 36] = [
        0, 1, 2, 0, 2, 3, // front
        4, 5, 6, 4, 6, 7, // back
        8, 9, 10, 8, 10, 11, // top
        12, 13, 14, 12, 14, 15, // bottom
        16, 17, 18, 16, 18, 19, // right
        20, 21, 22, 20, 22, 23, // left
    ];
    let index_array = uint_16_array!(indices);
    context.buffer_data_with_array_buffer_view(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        &index_array,
        WebGlRenderingContext::STATIC_DRAW,
    );

    Ok(Buffers(position_buffer, color_buffer, index_buffer))
}


#[allow(dead_code)]
fn draw_scene(
    gl: &WebGlRenderingContext,
    programInfo: ProgramInfo,
    buffers: Buffers,
    theta: f32,
    phi: f32,
) -> Result<(), JsValue> {
    use std::f32::consts::PI;
    let Buffers(positionBuffer, colorBuffer, indexBuffer) = buffers;
    let ProgramInfo(
        shaderProgram,
        (vertexPosition, vertexColor),
        (location_projectionMatrix, location_modelViewMatrix),
    ) = programInfo;
    gl.clear_color(0.0, 0.0, 0.0, 1.0); // Clear to black, fully opaque
    gl.clear_depth(1.0); // Clear everything
    gl.enable(WebGlRenderingContext::DEPTH_TEST); // Enable depth testing
                                                  // gl.depth_func(WebGlRenderingContext::LEQUAL); // Near things obscure far things

    // Clear the canvas before we start drawing on it.

    gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT | WebGlRenderingContext::DEPTH_BUFFER_BIT);
    // Create a perspective matrix, a special matrix that is
    // used to simulate the distortion of perspective in a camera.
    // Our field of view is 45 degrees, with a width/height
    // ratio that matches the display size of the canvas
    // and we only want to see objects between 0.1 units
    // and 100 units away from the camera.

    let fieldOfView = 45.0 * PI / 180.0; // in radians
    let canvas: web_sys::HtmlCanvasElement = gl
        .canvas()
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    gl.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);
    let aspect: f32 = canvas.width() as f32 / canvas.height() as f32;
    let zNear = 1.0;
    let zFar = 100.0;
    let mut projectionMatrix = mat4::new_zero();

    mat4::perspective(&mut projectionMatrix, &fieldOfView, &aspect, &zNear, &zFar);

    // Set the drawing position to the "identity" point, which is
    // the center of the scene.
    let mut modelViewMatrix = mat4::new_identity();

    // Now move the drawing position a bit to where we want to
    // start drawing the square.
    let mat_to_translate = modelViewMatrix.clone();
    mat4::translate(
        &mut modelViewMatrix, // destination matrix
        &mat_to_translate,    // matrix to translate
        &[-0.0, 0.0, -6.0],
    ); // amount to translate

    let mat_to_rotate = modelViewMatrix.clone();
    mat4::rotate_x(
        &mut modelViewMatrix, // destination matrix
        &mat_to_rotate,       // matrix to rotate
        &phi,
    );
    let mat_to_rotate = modelViewMatrix.clone();
    mat4::rotate_y(
        &mut modelViewMatrix, // destination matrix
        &mat_to_rotate,       // matrix to rotate
        &theta,
    );

    // Tell WebGL how to pull out the positions from the position
    // buffer into the vertexPosition attribute
    {
        let numComponents = 3;
        let type_ = WebGlRenderingContext::FLOAT;
        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&positionBuffer));

        gl.vertex_attrib_pointer_with_i32(
            vertexPosition,
            numComponents,
            type_,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(vertexPosition);
        // gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);
    }
    // Tell WebGL how to pull out the colors from the color buffer
    // into the vertexColor attribute.
    {
        let numComponents = 4;
        let type_ = WebGlRenderingContext::FLOAT;
        let normalize = false;
        let stride = 0;
        let offset = 0;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&colorBuffer));
        gl.vertex_attrib_pointer_with_i32(
            vertexColor,
            numComponents,
            type_,
            normalize,
            stride,
            offset,
        );
        gl.enable_vertex_attrib_array(vertexColor);

        // gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, None);
    }

    // Tell WebGL which indices to use to index the vertices
    gl.bind_buffer(
        WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
        Some(&indexBuffer),
    );

    // Tell WebGL to use our program when drawing

    gl.use_program(Some(&shaderProgram));

    // Set the shader uniforms

    gl.uniform_matrix4fv_with_f32_array(
        Some(&location_projectionMatrix?),
        false,
        &projectionMatrix,
    );
    gl.uniform_matrix4fv_with_f32_array(Some(&location_modelViewMatrix?), false, &modelViewMatrix);
    {
        let vertexCount = 36;
        let type_ = WebGlRenderingContext::UNSIGNED_SHORT;
        let offset = 0;
        gl.draw_elements_with_i32(WebGlRenderingContext::TRIANGLES, vertexCount, type_, offset);
    }

    Ok(())
}

pub fn request_animation_frame(f: &Closure<FnMut(f32)>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}
