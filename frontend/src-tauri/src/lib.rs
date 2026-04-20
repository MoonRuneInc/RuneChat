#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let _ = env_logger::try_init();
  log::info!("RuneChat starting...");

  let mut builder = tauri::Builder::default()
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_websocket::init())
    .setup(|_app| {
      log::info!("Setup callback running");
      Ok(())
    });

  #[cfg(feature = "updater")]
  {
    builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
  }

  builder
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
