use std::{error::Error, process::Command};
use std::thread;
use std::time::Duration;
use wgpu_glyph::{ab_glyph, GlyphBrushBuilder, Section, Text};
use systemstat::{System, Platform, saturating_sub_bytes};

pub const W: u32 = 160;
pub const H: u32 = 280;

fn main() -> Result<(), Box<dyn Error>> {
    // Open window and create a surface
    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_title("Benchmarco")
        .with_inner_size(winit::dpi::PhysicalSize {width: W, height: H})
        .with_position(winit::dpi::LogicalPosition {x: 0, y: 0})
        .with_resizable(false)
        .with_always_on_top(true)
        .with_decorations(false)
        .with_transparent(true)
        .build(&event_loop)
        .unwrap();
    if let Some(monitor) = window.current_monitor() {
        window.set_outer_position(winit::dpi::LogicalPosition {x: monitor.size().width - W, y: 0});
    }

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };

    let (device, queue) = futures::executor::block_on(async {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Request adapter")
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("Request device")
    });

    // Create staging belt and a local pool
    let mut staging_belt = wgpu::util::StagingBelt::new(1024);

    // Prepare swap chain
    let render_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: render_format,
            width: W,
            height: H,
            present_mode: wgpu::PresentMode::Fifo,
        },
    );

    // Prepare glyph_brush
    let font = ab_glyph::FontArc::try_from_slice(include_bytes!("../assets/font.ttf"))?;

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build(&device, render_format);

    let sys = System::new();

    window.request_redraw();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { ref event, .. } => match event {
                winit::event::WindowEvent::CloseRequested |
                winit::event::WindowEvent::KeyboardInput {
                    input: winit::event::KeyboardInput {
                        state: winit::event::ElementState::Pressed,
                        virtual_keycode: Some(winit::event::VirtualKeyCode::Escape), ..
                    }, ..
                } => *control_flow = winit::event_loop::ControlFlow::Exit,
                winit::event::WindowEvent::MouseInput { button: winit::event::MouseButton::Left, .. } =>
                    window.set_minimized(true),
                winit::event::WindowEvent::MouseInput { button: winit::event::MouseButton::Right, .. } =>
                    *control_flow = winit::event_loop::ControlFlow::Exit,
                winit::event::WindowEvent::Resized(new_size) => 
                    surface.configure(
                        &device,
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: render_format,
                            width: new_size.width,
                            height: new_size.height,
                            present_mode: wgpu::PresentMode::Mailbox
                        }
                    ),
                _ => {}
            }
            winit::event::Event::RedrawRequested { .. } => {
                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None }
                );
                let frame = surface.get_current_texture().unwrap();
                let view = &frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Clear frame
                {
                    let _ = encoder.begin_render_pass(
                        &wgpu::RenderPassDescriptor {
                            label: Some("Render pass"),
                            color_attachments: &[
                                wgpu::RenderPassColorAttachment {
                                    view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(
                                            wgpu::Color {
                                                r: 0.0,
                                                g: 0.0,
                                                b: 0.0,
                                                a: 0.2
                                            }
                                        ),
                                        store: true
                                    }
                                }
                            ],
                            depth_stencil_attachment: None
                        }
                    );
                }

                let res = format!("GPU\n{}\n\nCPU\nusg  {}\ntmp  {}\n\nRAM\nusg  {}",
                    match Command::new("nvidia-smi").args(vec!["-q"]).output() {
                        Ok(res) => {
                            let res = String::from_utf8_lossy(&res.stdout);
                            let lines: Vec<&str> = res.split('\n').collect();
                            let fan = String::from_utf8_lossy(&get_value(&lines[66].as_bytes(), 2));
                            let mem_total = String::from_utf8_lossy(&get_value(&lines[79].as_bytes(), 4)).parse::<u32>().unwrap();
                            let mem_free = String::from_utf8_lossy(&get_value(&lines[82].as_bytes(), 4)).parse::<u32>().unwrap();
                            let mem_used = mem_total - mem_free;
                            let usage = String::from_utf8_lossy(&get_value(&lines[89].as_bytes(), 2));
                            let fps = String::from_utf8_lossy(&get_value(&lines[95].as_bytes(), 0));
                            let tmp = String::from_utf8_lossy(&get_value(&lines[121].as_bytes(), 2));
                            let clock_graphics = String::from_utf8_lossy(&get_value(&lines[137].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_graphics_max = String::from_utf8_lossy(&get_value(&lines[148].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_sm = String::from_utf8_lossy(&get_value(&lines[138].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_sm_max = String::from_utf8_lossy(&get_value(&lines[149].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_mem = String::from_utf8_lossy(&get_value(&lines[139].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_mem_max = String::from_utf8_lossy(&get_value(&lines[150].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_video = String::from_utf8_lossy(&get_value(&lines[140].as_bytes(), 4)).parse::<u32>().unwrap();
                            let clock_video_max = String::from_utf8_lossy(&get_value(&lines[151].as_bytes(), 4)).parse::<u32>().unwrap();
                            format!("usg  {}%\ntmp  {}°C\nfps  {}\nfan  {}%\nmem  {}%  {}/{}\nclock\n  gpc  {: >2}%  {}/{}\n  sm   {: >2}%  {}/{}\n\
                            \t  mem  {: >2}%  {}/{}\n  vdo  {: >2}%  {}/{}",
                                usage, tmp, fps, fan, mem_total/mem_used, mem_used, mem_total,
                                clock_graphics_max/clock_graphics, clock_graphics, clock_graphics_max,
                                clock_mem_max/clock_mem, clock_mem, clock_mem_max,
                                clock_sm_max/clock_sm, clock_sm, clock_sm_max, 
                                clock_video_max/clock_video, clock_video, clock_video_max)
                        },
                        Err(e) => format!("Error: {}", e)
                    },
                    match sys.cpu_load_aggregate() {
                        Ok(cpu)=> {
                            thread::sleep(Duration::from_millis(500));
                            let cpu = cpu.done().unwrap();
                            format!("{:.0}%", ((cpu.user + cpu.system) * 100.0))
                        },
                        Err(e) => format!("Error: {}", e)
                    },
                    match sys.cpu_temp() {
                        Ok(tmp) => format!("{:.0}°C", tmp),
                        Err(e) => format!("Error: {}", e)
                    },
                    match sys.memory() {
                        Ok(mem) => {
                            format!("{}/{}", saturating_sub_bytes(mem.total, mem.free), mem.total)
                        },
                        Err(e) => format!("Error: {}", e)
                    }
                );

                glyph_brush.queue(Section {
                    screen_position: (10.0, 5.0),
                    bounds: (W as f32, H as f32),
                    text: vec![Text::new(&res)
                        .with_color([1.0, 1.0, 1.0, 1.0])
                        .with_scale(15.0)],
                    layout: wgpu_glyph::Layout::default_wrap()
                });

                glyph_brush.draw_queued(
                    &device,
                    &mut staging_belt,
                    &mut encoder,
                    view,
                    W,
                    H,
                ).unwrap();

                staging_belt.finish();
                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            winit::event::Event::MainEventsCleared => window.request_redraw(),
            _ => *control_flow = winit::event_loop::ControlFlow::Wait
        }
    })
}

fn get_value(line: &[u8], offset: usize) -> &[u8] {
    let f = line.len()-offset;
    let mut i = f-1;
    while line[i] != b' ' {
        i -= 1;
    }
    &line[i+1..f]
}