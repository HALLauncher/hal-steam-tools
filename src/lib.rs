use log::{error, info};
use tauri::{plugin::{Builder, TauriPlugin}, Manager, Runtime};

pub struct SteamWorks {
  pub client: tokio::sync::Mutex<steamworks::Client>, 
  pub single_client: tokio::sync::Mutex<steamworks::SingleClient>
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("hal-steamworks")
  .setup(|app| {
    let (client, single) = steamworks::Client::init_app(394360).map_err(|err| {
      error!("Steamworks init failed: {}", err);
      err
    })?;

    app.manage(SteamWorks {
      client: tokio::sync::Mutex::new(client),
      single_client: tokio::sync::Mutex::new(single)
    });

    Ok(())
  })
  .on_event(|app, e| {
    match e {
      tauri::RunEvent::Resumed => {
        info!("Resumed");
      },
      tauri::RunEvent::WindowEvent { label, event, .. } => {
        info!("WindowEvent: {} {:?}", label, event);
      },
      _ => {}
    }
  })
  .build()
}
