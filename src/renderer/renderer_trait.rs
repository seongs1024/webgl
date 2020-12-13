use crate::shader::Shader;

use js_sys::WebAssembly;
use wasm_bindgen::JsCast;
use web_sys::WebGlRenderingContext as GL;

pub trait Renderer {
    fn shader(&self) -> Shader;

    fn render(&self, context: &GL);

    fn buffer_attributes(&self, context: &GL);

    fn buffer_f32_data(
        context: &GL,
        data: &[f32],
        attrib: u32,
        num_components: i32)
    {
        let normalize = false;
        let stride = 0;
        let offset = 0;
        let data_array = float_32_array!(data)
        let buffer = context
            .create_buffer()
            .ok_or("failed to create_buffer")?;

        context.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
        context.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &data_array,
            GL::STATIC_DRAW);
        context.vertex_attrib_pointer_with_i32(
            attrib,
            num_components,
            GL::FLOAT,
            normalize,
            stride,
            offset);
    }

    fn buffer_u8_data(gl: &GL, data: &[u8], attrib: u32, size: i32) {
        let memory_buffer = wasm_bindgen::memory()
            .dyn_into::<WebAssembly::Memory>()
            .unwrap()
            .buffer();

        let data_location = data.as_ptr() as u32;

        let data_array = js_sys::Uint8Array::new(&memory_buffer)
            .subarray(data_location, data_location + data.len() as u32);

        let buffer = gl.create_buffer().unwrap();

        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&buffer));
        gl.buffer_data_with_array_buffer_view(
            GL::ARRAY_BUFFER,
            &data_array,
            GL::STATIC_DRAW);
        gl.vertex_attrib_pointer_with_i32(
            attrib,
            size,
            GL::UNSIGNED_BYTE,
            false,
            0,
            0);
    }

    fn buffer_u16_indices(context: &GL, indices: &[u16]) {
        let indices_array = uint_16_array!(indices);
        let index_buffer = context
            .create_buffer()
            .ok_or("failed to create index buffer")?;
        context.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
        context.buffer_data_with_array_buffer_view(
            GL::ELEMENT_ARRAY_BUFFER,
            &indices_array,
            GL::STATIC_DRAW,
        );
    }

    macro_rules! float_32_array {
        ($arr:expr) => {{
            let memory_buffer = wasm_bindgen::memory()
                .dyn_into::<WebAssembly::Memory>()?
                .buffer();
            let arr_location = $arr.as_ptr() as u32 / 4;
            js_sys::Float32Array::new(&memory_buffer)
                .subarray(arr_location, arr_location + $arr.len() as u32)
        }};
    }

    macro_rules! uint_16_array {
        ($arr:expr) => {{
            let memory_buffer = wasm_bindgen::memory()
                .dyn_into::<WebAssembly::Memory>()?
                .buffer();
            let arr_location = $arr.as_ptr() as u32 / 2;
            js_sys::Uint16Array::new(&memory_buffer)
                .subarray(arr_location, arr_location + $arr.len() as u32)
        }};
    }
}
