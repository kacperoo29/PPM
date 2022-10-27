mod image;
mod jpeg;
mod ppm;

use gloo_events::EventListener;
use jpeg::JPEG;
use js_sys::{Uint8Array, Float32Array};
use ppm::PPM;
use wasm_bindgen::JsCast;
use web_sys::{WebGl2RenderingContext as GL, HtmlElement, HtmlInputElement, Blob, Url};
use web_sys::{
    window, CanvasRenderingContext2d, HtmlCanvasElement, WebGl2RenderingContext,
};
use yew::prelude::*;

use crate::image::{BitmapData, Image};

struct App {
    image: Option<Box<dyn Image>>,
    scale: f64,
    translate_pos: (f64, f64),
    file_changed: bool,
    quality: u8,
}

#[derive(Debug, Clone, PartialEq)]
enum Msg {
    LoadFile { value: Vec<u8> },
    Zoom { pos: (f64, f64), y_delta: f64 },
    Draw,
    MouseOver { pos: (f64, f64) },
    SaveAsJpeg,
    QualityChange { value: u8 },
    None,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            image: None,
            scale: 1.0,
            translate_pos: (0.0, 0.0),
            file_changed: false,
            quality: 100,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let file_cb = ctx
            .link()
            .callback(|value: Vec<u8>| Msg::LoadFile { value });
        html! {
            <div>
                <div>
                    <input type="file" onchange={ctx.link().callback(move |event: Event| {
                        let file_cb = file_cb.clone();
                        let target = event.target().unwrap();
                        let target: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                        let file = target.files().unwrap().get(0).unwrap();
                        let file_reader = web_sys::FileReader::new().unwrap();
                        file_reader.read_as_array_buffer(&file).unwrap();
                        let listener = EventListener::new(&file_reader, "load", move |event| {
                            let target = event.target().unwrap();
                            let target: web_sys::FileReader = target.dyn_into().unwrap();
                            let result = target.result().unwrap();
                            let array = Uint8Array::new(&result);

                            file_cb.emit(array.to_vec());
                        });
                        listener.forget();

                        Msg::None
                    })} />
                    <label>{"Quality: "}</label>
                    <input type="range" min="0" max="100" value={self.quality.to_string()} step="1" onchange={ctx.link().callback(|event: Event| {
                        let quality = event.target().unwrap().dyn_into::<HtmlInputElement>().unwrap().value_as_number();

                        Msg::QualityChange { value: quality as u8 }
                    } )} />
                    <span>{self.quality.to_string()}</span>
                    <input type="button" value="Save as jpeg" onclick={ctx.link().callback(|_| Msg::SaveAsJpeg)} />
                    <span id="prompt" style="display: none;" />
                </div>
                <div style="overflow: auto; width: 95vw; height: 90vh;"
                    onwheel={ctx.link().callback(|event: WheelEvent| {
                    event.prevent_default();

                    Msg::Zoom { pos: (event.offset_x() as f64, event.offset_y() as f64), y_delta: event.delta_y() }
                })}>
                    <canvas id="canvas" width="0" height="0"
                        onmousemove={ctx.link().callback(|event: MouseEvent|
                            Msg::MouseOver { pos: (event.offset_x() as f64,event.offset_y() as f64)
                    })} />
                </div>
            </div>
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let canvas = window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector("#canvas")
            .unwrap()
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        let rendering_context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();

        match msg {
            Msg::LoadFile { value } => {
                // Check if jpeg or ppm
                if value[0] == 0xFF && value[1] == 0xD8 {
                    self.image = Some(Box::new(JPEG::from_buffer(&mut value.clone())));
                } else {
                    self.image = Some(Box::new(PPM::from_buffer(&mut value.clone())));
                }

                self.file_changed = true;
                self.scale = 1.0;
                ctx.link().send_message(Msg::Draw);

                true
            }
            Msg::Zoom { pos, y_delta } => {
                let scale = if y_delta > 0.0 {
                    self.scale * 0.9
                } else {
                    self.scale * 1.1
                };

                let translate_pos = (
                    self.translate_pos.0
                        + (pos.0 - self.translate_pos.0) * (1.0 - scale / self.scale),
                    self.translate_pos.1
                        + (pos.1 - self.translate_pos.1) * (1.0 - scale / self.scale),
                );

                self.scale = scale;
                self.translate_pos = translate_pos;

                ctx.link().send_message(Msg::Draw);

                true
            }
            Msg::Draw => {
                if self.image.is_none() {
                    return false;
                }

                let ppm = self.image.as_ref().unwrap();
                canvas.set_width(ppm.get_width() as u32);
                canvas.set_height(ppm.get_height() as u32);

                let new_canvas = match window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .query_selector("#new_canvas")
                {
                    Ok(Some(canvas)) => canvas,
                    _ => {
                        let canvas = window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .create_element("canvas")
                            .unwrap();
                        canvas.set_attribute("id", "new_canvas").unwrap();
                        canvas.set_attribute("style", "display: none;").unwrap();
                        window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .body()
                            .unwrap()
                            .append_child(&canvas)
                            .unwrap();
                        canvas
                    }
                }
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();

                if self.file_changed {
                    new_canvas.set_width(ppm.get_width() as u32);
                    new_canvas.set_height(ppm.get_height() as u32);

                    let glctx = new_canvas
                        .get_context("webgl2")
                        .unwrap()
                        .unwrap()
                        .dyn_into::<WebGl2RenderingContext>()
                        .unwrap();
                    glctx.viewport(0, 0, ppm.get_width() as i32, ppm.get_height() as i32);

                    let texture = glctx.create_texture();
                    glctx.bind_texture(GL::TEXTURE_2D, texture.as_ref());
                    glctx.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR as i32);
                    glctx.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
                    glctx.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
                    glctx.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
                    glctx.pixel_storei(GL::UNPACK_ALIGNMENT, 1);

                    match &ppm.get_buffer_ref() {
                        BitmapData::U8(data) => {
                            glctx.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                                GL::TEXTURE_2D, 
                                0, 
                                GL::RGB8 as i32, 
                                ppm.get_width() as i32, 
                                ppm.get_height() as i32, 
                                0, 
                                GL::RGB, 
                                GL::UNSIGNED_BYTE, 
                                Some(&data))
                            .expect("Couldn't load texture data.");
                        }
                        BitmapData::U16(data) => {
                            let data: Vec<f32> = data.iter().map(|val| (*val as f32) / u16::MAX as f32).collect();
                            let array = Float32Array::from(data.as_slice());
                            glctx.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                                GL::TEXTURE_2D, 
                                0, 
                                GL::RGB16F as i32, 
                                ppm.get_width() as i32, 
                                ppm.get_height() as i32, 
                                0, 
                                GL::RGB, 
                                GL::FLOAT, 
                                Some(&array))
                            .expect("Couldn't load texture data.");
                        }
                        BitmapData::None => {},                        
                    };

                    let vertex_shader = glctx
                        .create_shader(GL::VERTEX_SHADER)
                        .expect("Unable to create vertex shader.");
                    glctx.shader_source(
                        &vertex_shader,
                        r#"#version 300 es
                        in vec2 a_position;
                        in vec2 a_texcoord;
                        out vec2 v_texcoord;
                        uniform vec2 u_translate_pos;
                        uniform float u_scale;
                        void main() {
                            gl_Position = vec4(a_position, 0.0, 1.0);
                            v_texcoord = a_texcoord;
                        }"#,
                    );
                    glctx.compile_shader(&vertex_shader);

                    let fragment_shader = glctx
                        .create_shader(GL::FRAGMENT_SHADER)
                        .expect("Unable to create fragment shader.");
                    glctx.shader_source(
                        &fragment_shader,
                        r#"#version 300 es
                        precision highp float;
                        in vec2 v_texcoord;
                        out vec4 outColor;
                        uniform sampler2D u_texture;
                        void main() {
                            outColor = texture(u_texture, v_texcoord);
                        }"#,
                    );
                    glctx.compile_shader(&fragment_shader);

                    let program = glctx
                        .create_program()
                        .expect("Unable to create shader program.");
                    glctx.attach_shader(&program, &vertex_shader);
                    glctx.attach_shader(&program, &fragment_shader);
                    glctx.link_program(&program);

                    let va = glctx.create_vertex_array();
                    glctx.bind_vertex_array(va.as_ref());

                    let buffer = glctx.create_buffer();
                    glctx.bind_buffer(GL::ARRAY_BUFFER, buffer.as_ref());
                    glctx.buffer_data_with_array_buffer_view(
                        GL::ARRAY_BUFFER,
                        &Float32Array::from([
                            -1.0f32, -1.0f32,   0.0f32, 1.0f32, 
                             1.0f32,  -1.0f32,  1.0f32, 1.0f32, 
                            -1.0f32,  1.0f32,   0.0f32, 0.0f32, 
                             1.0f32,   1.0f32,  1.0f32, 0.0f32,
                        ].as_slice()),
                        GL::STATIC_DRAW,
                    );
                    glctx.vertex_attrib_pointer_with_i32(0, 2, GL::FLOAT, false, 16, 0);
                    glctx.enable_vertex_attrib_array(0);
                    glctx.vertex_attrib_pointer_with_i32(1, 2, GL::FLOAT, false, 16, 8);
                    glctx.enable_vertex_attrib_array(1);

                    glctx.clear_color(0.0, 0.0, 0.0, 1.0);
                    glctx.clear(GL::COLOR_BUFFER_BIT);
                    glctx.use_program(Some(&program));
                    glctx.bind_vertex_array(va.as_ref());
                    glctx.bind_texture(GL::TEXTURE_2D, texture.as_ref());
                    glctx.draw_arrays(GL::TRIANGLE_STRIP, 0, 4);

                    self.file_changed = false;
                }

                let mut scaled_width = (ppm.get_width() as f64 * self.scale) as u32;
                let mut scaled_height = (ppm.get_height() as f64 * self.scale) as u32;
                const MAX_CANVAS: u32 = 19000;
                scaled_width = if scaled_width > MAX_CANVAS { MAX_CANVAS } else { scaled_width };
                scaled_height = if scaled_height > MAX_CANVAS { MAX_CANVAS } else { scaled_height };
                let scale = if scaled_width == MAX_CANVAS || scaled_height == MAX_CANVAS {
                    let scale_x = MAX_CANVAS as f64 / ppm.get_width() as f64;
                    let scale_y = MAX_CANVAS as f64 / ppm.get_height() as f64;
                    if scale_x < scale_y { scale_x } else { scale_y }
                } else {
                    self.scale
                };

                canvas.set_width((ppm.get_width() as f64 * scale) as u32);
                canvas.set_height((ppm.get_height() as f64 * scale) as u32);

                rendering_context.clear_rect(
                    0.0,
                    0.0,
                    scaled_width as f64,
                    scaled_height as f64,
                );

                rendering_context.set_image_smoothing_enabled(false);
                rendering_context.translate(0.0, 0.0);
                rendering_context.scale(scale, scale);
                rendering_context.draw_image_with_html_canvas_element(&new_canvas, 0.0, 0.0);

                true
            }
            Msg::None => false,
            Msg::MouseOver { pos } => {
                let prompt = window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .query_selector("#prompt")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<HtmlElement>()
                    .unwrap();
                
                let scaled_x = (pos.0 / self.scale).floor() as usize;
                let scaled_y = (pos.1 / self.scale).floor() as usize;
                log::info!("Mouse over: {}, {}", scaled_x, scaled_y);
                let ppm = self.image.as_ref().unwrap();
                // check if in bounds
                if !(scaled_x < ppm.get_width() && scaled_y < ppm.get_height()) {
                    prompt.set_attribute("style", &format!("display: none;"))
                    .unwrap();
                }

                let (r, g, b) = ppm.get_pixel_value(scaled_x, scaled_y);
                let text = format!("r: {}, g: {}, b: {}", r, g, b);
                prompt.set_inner_text(&text);
                prompt.set_attribute("style", &format!("left: {}px; top: {}px; display: block-inline;", pos.0, pos.1))
                    .unwrap();

                true
            },
            Msg::SaveAsJpeg => {
                if self.image.is_none() {
                    return false;
                }

                let image = self.image.as_ref().unwrap();
                let mut vec = Vec::new();
                image.write_to_jpeg(&mut vec, self.quality).expect("Unable to write to jpeg");

                let a = window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .create_element("a")
                    .unwrap()
                    .dyn_into::<HtmlElement>()
                    .unwrap();

                let blob = Blob::new_with_u8_array_sequence(&Uint8Array::from(&vec[..])).unwrap();
                a.set_attribute("href", &format!("data:image/jpeg;base64,{}", base64::encode(&vec[..])))
                    .unwrap();
                a.set_attribute("download", "image.jpeg").unwrap();
                
                a.click();
                a.remove();

                true
            },
            Msg::QualityChange { value } => {
                self.quality = value;

                true
            },
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            return;
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<App>();
}
