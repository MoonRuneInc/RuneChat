#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let _ = env_logger::try_init();
  log::info!("Cauldron starting...");

  let mut builder = tauri::Builder::default()
    .plugin(tauri_plugin_http::init())
    .plugin(tauri_plugin_websocket::init())
    // Workaround for Tauri v2 internal tauri:// protocol handler failing on
    // Linux/WSL with WebKitGTK 2.50. By registering our own handler, we bypass
    // Tauri's asset serving logic and serve embedded assets directly.
    .register_uri_scheme_protocol("tauri", |ctx, req| {
      let path = req.uri().path().trim_start_matches('/').to_string();
      let path = if path.is_empty() { "index.html".to_string() } else { path };
      if let Some(asset) = ctx.app_handle().asset_resolver().get(path.clone()) {
        tauri::http::Response::builder()
          .header("Content-Type", asset.mime_type)
          .body(asset.bytes)
          .unwrap()
      } else {
        tauri::http::Response::builder()
          .status(404)
          .header("Content-Type", "text/plain")
          .body(format!("Not found: {}", path).into_bytes())
          .unwrap()
      }
    })
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
