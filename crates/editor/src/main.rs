// Point d'entrée de l'éditeur et du lecteur autonome.
// Initialise winit, wgpu et egui, puis lance la boucle d'application.

use std::sync::Arc;
use std::time::Instant;

use egui_wgpu::ScreenDescriptor;
use wgpu::SurfaceError;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use editor::EditorApp;

fn main() {
    // Initialise le logger (affiche dans la console).
    let mut logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    // Keep engine info logs while hiding wgpu's per-frame submission waits.
    logger.filter_module("wgpu_core::device::resource", log::LevelFilter::Warn);
    logger.init();

    log::info!("=== Moteur de jeu 2D isométrique ===");

    let is_player = env!("CARGO_BIN_NAME") == "player";
    let initial_editor = if is_player {
        prepare_bundled_python();
        let project_path = std::env::args_os().nth(1).map(std::path::PathBuf::from)
            .or_else(find_project_next_to_executable);
        let Some(project_path) = project_path else {
            report_startup_error("Impossible de trouver le projet à ouvrir. Passez le chemin .isomaker en argument.");
            return;
        };
        match EditorApp::new_player(&project_path) {
            Ok(editor) => Some(editor),
            Err(error) => {
                report_startup_error(&format!("Impossible de lancer le jeu depuis {} : {error}", project_path.display()));
                return;
            }
        }
    } else {
        None
    };

    let event_loop = EventLoop::new().expect("Impossible de créer l'event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::Uninitialized { is_player, editor: initial_editor };
    event_loop.run_app(&mut app).expect("Erreur de boucle d'événements");
}

#[cfg(windows)]
fn report_startup_error(message: &str) {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    let title: Vec<u16> = std::ffi::OsStr::new("ISOMaker — erreur de lancement")
        .encode_wide().chain(std::iter::once(0)).collect();
    let text: Vec<u16> = std::ffi::OsStr::new(message)
        .encode_wide().chain(std::iter::once(0)).collect();
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: *mut c_void, text: *const u16, title: *const u16, kind: u32) -> i32;
    }
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10); }
}

#[cfg(not(windows))]
fn report_startup_error(message: &str) {
    eprintln!("{message}");
}

fn find_project_next_to_executable() -> Option<std::path::PathBuf> {
    let directory = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let default = directory.join("game.isomaker");
    if default.is_file() {
        return Some(default);
    }
    let mut projects = std::fs::read_dir(directory).ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("isomaker")));
    let project = projects.next()?;
    projects.next().is_none().then_some(project)
}

fn prepare_bundled_python() {
    let Some(home) = std::env::current_exe().ok()
        .and_then(|executable| executable.parent().map(|directory| directory.join("python-runtime")))
        .filter(|home| home.join("Lib").is_dir()) else {
            return;
        };
    std::env::set_var("PYTHONHOME", &home);
    let mut paths = vec![home.clone(), home.join("DLLs")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    if let Ok(path) = std::env::join_paths(paths) {
        std::env::set_var("PATH", path);
    }
}

// ── États de l'application ────────────────────────────────────────────────────

enum App {
    Uninitialized { is_player: bool, editor: Option<EditorApp> },
    Running(RunningApp),
}

struct RunningApp {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // egui
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    // Application
    editor: EditorApp,
    last_frame: Instant,
    ctrl_down: bool,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if matches!(self, App::Running(_)) {
            return; // Déjà initialisé.
        }
        let is_player = matches!(self, App::Uninitialized { is_player: true, .. });

        // ── Fenêtre ────────────────────────────────────────────────────────────
        let title = if is_player { "ISOMaker — Jeu" } else { "Moteur de jeu 2D isométrique — Éditeur" };
        let attrs = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1400u32, 900u32));
        let window = Arc::new(
            event_loop.create_window(attrs).expect("Impossible de créer la fenêtre"),
        );

        // ── wgpu ──────────────────────────────────────────────────────────────
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // SAFETY : la surface vit aussi longtemps que la fenêtre (Arc).
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("Impossible de créer la surface wgpu");

        let (device, queue, surface_config, surface_format) =
            pollster::block_on(init_wgpu(&instance, &surface, window.inner_size()));

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // ── egui ─────────────────────────────────────────────────────────────
        let egui_ctx = egui::Context::default();

        // Style sombre.
        egui_ctx.set_visuals(egui::Visuals::dark());

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui_ctx.viewport_id(),
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            None,
            1,
            false,
        );

        // ── Éditeur ───────────────────────────────────────────────────────────
        let editor = match self {
            App::Uninitialized { editor, .. } => editor.take().unwrap_or_else(EditorApp::new),
            App::Running(_) => unreachable!(),
        };

        log::info!("Éditeur initialisé. Fenêtre {}×{}.",
            window.inner_size().width, window.inner_size().height);

        *self = App::Running(RunningApp {
            window,
            surface,
            surface_config,
            device,
            queue,
            egui_ctx,
            egui_state,
            egui_renderer,
            editor,
            last_frame: Instant::now(),
            ctrl_down: false,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let App::Running(app) = self else { return };

        if let WindowEvent::ModifiersChanged(modifiers) = &event {
            app.ctrl_down = modifiers.state().control_key();
        }

        // Intercepte les raccourcis d'édition avant leur traitement par egui-winit.
        if app.ctrl_down && !app.egui_ctx.wants_keyboard_input() {
            if let WindowEvent::KeyboardInput { event: key_event, .. } = &event {
                if key_event.state == ElementState::Pressed && !key_event.repeat {
                    let shortcut = match key_event.physical_key {
                        PhysicalKey::Code(KeyCode::KeyC) => Some(Shortcut::Copy),
                        PhysicalKey::Code(KeyCode::KeyV) => Some(Shortcut::Paste),
                        _ => None,
                    };
                    if let Some(shortcut) = shortcut {
                        let handled = match shortcut {
                            Shortcut::Copy => app.editor.queue_clipboard_shortcut(true),
                            Shortcut::Paste => app.editor.queue_clipboard_shortcut(false),
                        };
                        if handled {
                            app.window.request_redraw();
                            return;
                        }
                    }
                }
            }
        }

        // L'entrée jeu est capturée avant que egui puisse consommer un raccourci.
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            if let Some(key) = map_logical_key(&event.logical_key).or_else(|| match event.physical_key {
                PhysicalKey::Code(code) => map_key(code),
                _ => None,
            }) {
                    match event.state {
                        ElementState::Pressed => app.editor.state.input.key_down(key),
                        ElementState::Released => app.editor.state.input.key_up(key),
                    }
            }
        }

        // Transmet l'événement à egui.
        let resp = app.egui_state.on_window_event(&app.window, &event);
        if resp.consumed && !matches!(event, WindowEvent::RedrawRequested | WindowEvent::CloseRequested) { return; }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Fermeture demandée.");
                if app.editor.request_close() {
                    app.editor.shutdown_player();
                    event_loop.exit();
                } else {
                    app.window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    app.surface_config.width  = size.width;
                    app.surface_config.height = size.height;
                    app.surface.configure(&app.device, &app.surface_config);
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(app.last_frame).as_secs_f32();
                app.last_frame = now;

                // Récupère la texture de rendu.
                let output = match app.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        app.surface.configure(&app.device, &app.surface_config);
                        return;
                    }
                    Err(SurfaceError::Timeout) => return,
                    Err(e) => {
                        log::error!("Erreur surface : {e}");
                        event_loop.exit();
                        return;
                    }
                };

                let view = output.texture.create_view(&Default::default());

                // ── Frame egui ─────────────────────────────────────────────────
                let raw_input = app.egui_state.take_egui_input(&app.window);
                let full_output = app.egui_ctx.run(raw_input, |ctx| {
                    app.editor.update(ctx, delta);
                });
                let close_app = app.editor.take_exit_request();
                // Les événements edge ne valent que pour ce callback de jeu.
                app.editor.state.input.begin_frame();

                app.egui_state.handle_platform_output(&app.window, full_output.platform_output);

                let tris = app.egui_ctx.tessellate(full_output.shapes, app.egui_ctx.pixels_per_point());
                for (id, delta) in full_output.textures_delta.set {
                    app.egui_renderer.update_texture(&app.device, &app.queue, id, &delta);
                }
                for id in full_output.textures_delta.free {
                    app.egui_renderer.free_texture(&id);
                }

                let screen_desc = ScreenDescriptor {
                    size_in_pixels: [app.surface_config.width, app.surface_config.height],
                    pixels_per_point: app.egui_ctx.pixels_per_point(),
                };

                let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_encoder"),
                });

                app.egui_renderer.update_buffers(
                    &app.device,
                    &app.queue,
                    &mut encoder,
                    &tris,
                    &screen_desc,
                );

                {
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui_render_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1, g: 0.12, b: 0.15, a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    let mut render_pass = render_pass.forget_lifetime();
                    app.egui_renderer.render(&mut render_pass, &tris, &screen_desc);
                }

                app.queue.submit(std::iter::once(encoder.finish()));
                output.present();

                if close_app {
                    app.editor.shutdown_player();
                    event_loop.exit();
                    return;
                }

                // Redemande un frame immédiatement (mode Poll).
                app.window.request_redraw();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let App::Running(app) = self {
            app.window.request_redraw();
        }
    }
}

#[derive(Clone, Copy)]
enum Shortcut {
    Copy,
    Paste,
}

fn map_key(code: KeyCode) -> Option<engine::Key> {
    use engine::Key as K;
    Some(match code {
        KeyCode::KeyA => K::A, KeyCode::KeyB => K::B, KeyCode::KeyC => K::C,
        KeyCode::KeyD => K::D, KeyCode::KeyE => K::E, KeyCode::KeyF => K::F,
        KeyCode::KeyG => K::G, KeyCode::KeyH => K::H, KeyCode::KeyI => K::I,
        KeyCode::KeyJ => K::J, KeyCode::KeyK => K::K, KeyCode::KeyL => K::L,
        KeyCode::KeyM => K::M, KeyCode::KeyN => K::N, KeyCode::KeyO => K::O,
        KeyCode::KeyP => K::P, KeyCode::KeyQ => K::Q, KeyCode::KeyR => K::R,
        KeyCode::KeyS => K::S, KeyCode::KeyT => K::T, KeyCode::KeyU => K::U,
        KeyCode::KeyV => K::V, KeyCode::KeyW => K::W, KeyCode::KeyX => K::X,
        KeyCode::KeyY => K::Y, KeyCode::KeyZ => K::Z,
        KeyCode::Digit0 => K::Key0, KeyCode::Digit1 => K::Key1, KeyCode::Digit2 => K::Key2,
        KeyCode::Digit3 => K::Key3, KeyCode::Digit4 => K::Key4, KeyCode::Digit5 => K::Key5,
        KeyCode::Digit6 => K::Key6, KeyCode::Digit7 => K::Key7, KeyCode::Digit8 => K::Key8,
        KeyCode::Digit9 => K::Key9,
        KeyCode::Escape => K::Escape, KeyCode::Enter | KeyCode::NumpadEnter => K::Enter,
        KeyCode::Space => K::Space, KeyCode::Backspace => K::Backspace,
        KeyCode::Delete => K::Delete, KeyCode::Tab => K::Tab,
        KeyCode::ArrowLeft => K::Left, KeyCode::ArrowRight => K::Right,
        KeyCode::ArrowUp => K::Up, KeyCode::ArrowDown => K::Down,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => K::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => K::Control,
        KeyCode::AltLeft | KeyCode::AltRight => K::Alt,
        KeyCode::F1 => K::F1, KeyCode::F2 => K::F2, KeyCode::F3 => K::F3,
        KeyCode::F4 => K::F4, KeyCode::F5 => K::F5, KeyCode::F6 => K::F6,
        KeyCode::F7 => K::F7, KeyCode::F8 => K::F8, KeyCode::F9 => K::F9,
        KeyCode::F10 => K::F10, KeyCode::F11 => K::F11, KeyCode::F12 => K::F12,
        _ => return None,
    })
}

/// Préfère le caractère logique pour que `keys.Z` désigne bien Z sur AZERTY
/// comme sur QWERTY, même quand Shift transforme le caractère en majuscule.
fn map_logical_key(key: &winit::keyboard::Key) -> Option<engine::Key> {
    let winit::keyboard::Key::Character(character) = key else { return None };
    let mut chars = character.chars();
    let first = chars.next()?.to_ascii_lowercase();
    if chars.next().is_some() { return None; }
    use engine::Key as K;
    Some(match first {
        'a' => K::A, 'b' => K::B, 'c' => K::C, 'd' => K::D, 'e' => K::E,
        'f' => K::F, 'g' => K::G, 'h' => K::H, 'i' => K::I, 'j' => K::J,
        'k' => K::K, 'l' => K::L, 'm' => K::M, 'n' => K::N, 'o' => K::O,
        'p' => K::P, 'q' => K::Q, 'r' => K::R, 's' => K::S, 't' => K::T,
        'u' => K::U, 'v' => K::V, 'w' => K::W, 'x' => K::X, 'y' => K::Y,
        'z' => K::Z,
        '0' => K::Key0, '1' => K::Key1, '2' => K::Key2, '3' => K::Key3,
        '4' => K::Key4, '5' => K::Key5, '6' => K::Key6, '7' => K::Key7,
        '8' => K::Key8, '9' => K::Key9,
        _ => return None,
    })
}

// ── Initialisation wgpu ───────────────────────────────────────────────────────

async fn init_wgpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
    size: winit::dpi::PhysicalSize<u32>,
) -> (wgpu::Device, wgpu::Queue, wgpu::SurfaceConfiguration, wgpu::TextureFormat) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Aucun adaptateur GPU compatible");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("MoteurDevice"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .expect("Impossible de créer le device GPU");

    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    log::info!("Adaptateur GPU : {} ({:?})", adapter.get_info().name, adapter.get_info().backend);

    (device, queue, config, surface_format)
}
