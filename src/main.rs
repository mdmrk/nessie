#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::CreationContext;
use egui::{Style, Visuals};
use nessie::{app::App, args::Args};

fn set_style(cc: &CreationContext) {
    let style = Style {
        visuals: Visuals::dark(),
        ..Default::default()
    };
    cc.egui_ctx.set_style(style);
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    const VERSION: &str = env!("VERSION");

    env_logger::init();

    let args: Args = argh::from_env();

    if args.version {
        println!("{}", VERSION);
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title(format!("Nessie {VERSION}"))
            .with_icon(nessie::ui::Ui::app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Nessie",
        options,
        Box::new(|cc| {
            set_style(cc);
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
                    set_style(cc);
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
