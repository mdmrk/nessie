#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nessie::{app::App, args::Args};

#[cfg(all(debug_assertions, not(target_arch = "wasm32")))]
fn start_puffin_server() {
    puffin::set_scopes_on(true);

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            #[expect(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use nessie::ui::Ui;

    env_logger::init();

    let args: Args = argh::from_env();

    #[cfg(debug_assertions)]
    if args.profiling {
        start_puffin_server();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_icon(Ui::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Nessie",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx, args)))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use std::panic;

    use eframe::wasm_bindgen::JsCast as _;

    panic::set_hook(Box::new(console_error_panic_hook::hook));
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    let options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let args = Args::default();
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(|cc| {
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    Ok(Box::new(App::new(&cc.egui_ctx, args)))
                }),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
