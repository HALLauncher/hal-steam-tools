
use tauri::{
    plugin::{Builder, TauriPlugin}, Manager, Runtime,
};

pub mod workshop;
pub mod filesystem;
pub struct SteamWorks {
    pub client: std::sync::Mutex<steamworks::Client>,
    pub single_client: std::sync::Mutex<steamworks::SingleClient>,
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("hal-steamworks")
        .setup(|app| {
            let (client, single) = steamworks::Client::init_app(394360)?;

            app.manage(SteamWorks {
                client: std::sync::Mutex::new(client),
                single_client: std::sync::Mutex::new(single),
            });

            let handle = app.app_handle();
            app.listen_global("need-wokrshop-item", move |event| {
                workshop::need_workshop_item(handle.clone(), event)
            });

            Ok(())
        })
        .on_event(|app, e| match e {
            tauri::RunEvent::MainEventsCleared => {
                if let Ok(sc) = app.state::<SteamWorks>().single_client.lock() {
                    sc.run_callbacks();
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![workshop::get_workshop_item])
        .build()
}
