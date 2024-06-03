use std::sync::{Arc, Condvar, Mutex};

use log::error;
use steamworks::PublishedFileId;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Event, Manager, Runtime,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WokrshopItem {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub preview: Option<String>,
}

pub struct SteamWorks {
    pub client: std::sync::Mutex<steamworks::Client>,
    pub single_client: std::sync::Mutex<steamworks::SingleClient>,
}

fn need_workshop_item<R: Runtime>(handle: tauri::AppHandle<R>, event: Event) {
    let Some(payload) = event.payload() else {
        error!("need-wokrshop-item payload is null {}", event.id());
        return;
    };

    let Ok(id) = payload.parse::<u64>() else {
        error!(
            "need-wokrshop-item payload is not a number {} {}",
            payload,
            event.id()
        );
        return;
    };

    let state = handle.state::<SteamWorks>();
    let Ok(client) = state.client.lock() else {
        error!("need-wokrshop-item client is null {}", event.id());
        return;
    };

    let handle = handle.app_handle();
    let result = client
        .ugc()
        .query_item(PublishedFileId(id))
        .map(move |query| {
            query.fetch(move |x| {
                let Ok(info) = x else {
                    let _ = x.inspect_err(|err| error!("{err}"));
                    return;
                };

                let preview = info.preview_url(0);
                let item = info.get(0).map(|item| WokrshopItem {
                    id: item.published_file_id.0,
                    name: item.title,
                    description: Some(item.description),
                    preview,
                });

                if item.is_none() {
                    error!("need-wokrshop-item item is null {}", event.id());
                    return;
                }

                let _ = handle.emit_all("got-wokrshop-item", item);
            });
        });

    if let Err(err) = result {
        error!("{err}");
    }
}

#[tauri::command]
async fn get_workshop_item<R: Runtime>(
    app: tauri::AppHandle<R>,
    id: u64,
) -> Result<WokrshopItem, String> {
    let state = app.state::<SteamWorks>();
    let condvar = Arc::new((Mutex::<Option<Result<WokrshopItem, String>>>::new(None), Condvar::new()));

    let result = if let Ok(client) = state.client.lock() {
        let condvar = condvar.clone();
        client
            .ugc()
            .query_item(PublishedFileId(id))
            .map(move |query| {
                query.fetch(move |x| {
                    let Ok(info) = x else {
                        let _ = x.inspect_err(move |err| {
                            *condvar.0.lock().unwrap() = Some(Err(err.to_string()));
                            condvar.1.notify_all();
                        });
                        return;
                    };

                    let preview = info.preview_url(0);
                    let item = info.get(0).map(|item| WokrshopItem {
                        id: item.published_file_id.0,
                        name: item.title,
                        description: Some(item.description),
                        preview,
                    });

                    if item.is_none() {
                        *condvar.0.lock().unwrap() = Some(Err("item is null".to_string()));
                        condvar.1.notify_all();
                        return;
                    }

                    *condvar.0.lock().unwrap() = Some(Ok(item.unwrap()));
                    condvar.1.notify_all();
                });
            })
            .map_err(|err| format!("Cannot query item: {err}"))
    } else {
        Err("client is null".to_string())
    };

    result?;

    let lock = condvar
        .1
        .wait_while(condvar.0.lock().unwrap(), |x| x.is_none())
        .unwrap();

    let result = lock.clone();
    result.unwrap()
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
                need_workshop_item(handle.clone(), event)
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
        .invoke_handler(tauri::generate_handler![get_workshop_item])
        .build()
}
