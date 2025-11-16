#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nessie::{app::App, args::Args};

#[cfg(debug_assertions)]
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

fn main() -> eframe::Result {
    env_logger::init();

    let args: Args = argh::from_env();

    #[cfg(debug_assertions)]
    if args.profiling {
        start_puffin_server();
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Nessie",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&args)))
        }),
    )
}
