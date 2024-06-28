use std::sync::{Arc, Condvar, Mutex};

use log::error;
use steamworks::PublishedFileId;
use tauri::{Event, Manager, Runtime};

use crate::SteamWorks;

/// Represents a wokrshop item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WokrshopItem {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub preview: Option<String>,
}


/// Tauri event to request a wokrshop item.
/// ```ts
/// import { emit, listen } from "@tauri-apps/api/event";
/// 
/// emit("need-wokrshop-item", 1337);
/// 
/// listen<any>("got-wokrshop-item", async (event) => {
///     console.log(event.payload);
/// });
/// ```
pub(crate) fn need_workshop_item<R: Runtime>(handle: tauri::AppHandle<R>, event: Event) {
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

/// Get a wokrshop item by id.
/// ```ts
/// import { invoke } from "@tauri-apps/api";
/// 
/// await invoke("get-workshop-item", 1337);
/// ```
#[tauri::command]
pub(crate) async fn get_workshop_item<R: Runtime>(
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
