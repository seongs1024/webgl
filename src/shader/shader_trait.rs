puse wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext, WebGlShader, WebGlProgram};

pub trait Shader {
    pub fn new(
        context: &WebGlRenderingContext,
        vert_shader: &str,
        frag_shader: &str,
    ) -> Result<Shader, JsValue>;

    pub fn program() -> &WebGlProgram;

    pub fn build_program(
        context: &WebGlRenderingContext,
        vert_shader: &str,
        frag_shader: &str,
    ) -> Result<WebGlProgram, String>{
         let vert_shader = compile_shader(
             &context,
             WebGlRenderingContext::VERTEX_SHADER,
             vert_shader)?;
        let frag_shader = compile_shader(
            &context,
            WebGlRenderingContext::FRAGMENT_SHADER,
            frag_shader)?;

        link_program(&gl, &vert_shader, &frag_shader)
    }

    fn compile_shader(
        context: &WebGlRenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, String> {
        let shader = context
            .create_shader(shader_type)
            .ok_or_else(|| "Unable to create shader object".to_string())?;
        context.shader_source(&shader, source);
        context.compile_shader(&shader);

        if context
            .get_shader_parameter(
                &shader,
                WebGlRenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(context
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| "Unknown error creating shader".to_string()))
        }
    }

    fn link_program(
        context: &WebGlRenderingContext,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
    ) -> Result<WebGlProgram, String> {
        let program = context
            .create_program()
            .ok_or_else(|| "Unable to create shader object".to_string())?;

        context.attach_shader(&program, vert_shader);
        context.attach_shader(&program, frag_shader);
        context.link_program(&program);

        if context
            .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            Err(context
                .get_program_info_log(&program)
                .unwrap_or_else(|| 
                    "Unknown error creating program object".to_string())
                )
        }
    }
}
