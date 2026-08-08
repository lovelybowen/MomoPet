use rdev::{Event, EventType, listen};
use serde::Serialize;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Runtime, command};

#[derive(Debug, Clone, Serialize)]
pub enum DeviceEventKind {
    MousePress,
    MouseRelease,
    MouseMove,
    KeyboardPress,
    KeyboardRelease,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceEvent {
    kind: DeviceEventKind,
    value: Value,
}

static LISTENER_STARTED: AtomicBool = AtomicBool::new(false);
static IS_ENABLED: AtomicBool = AtomicBool::new(false);

#[command]
pub fn start_device_listening<R: Runtime>(app_handle: AppHandle<R>) {
    IS_ENABLED.store(true, Ordering::SeqCst);

    if LISTENER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let result = listen(move |event: Event| {
            if !IS_ENABLED.load(Ordering::SeqCst) {
                return;
            }

            let device_event = match event.event_type {
                EventType::ButtonPress(button) => DeviceEvent {
                    kind: DeviceEventKind::MousePress,
                    value: json!(format!("{button:?}")),
                },
                EventType::ButtonRelease(button) => DeviceEvent {
                    kind: DeviceEventKind::MouseRelease,
                    value: json!(format!("{button:?}")),
                },
                EventType::MouseMove { x, y } => DeviceEvent {
                    kind: DeviceEventKind::MouseMove,
                    value: json!({ "x": x, "y": y }),
                },
                EventType::KeyPress(key) => DeviceEvent {
                    kind: DeviceEventKind::KeyboardPress,
                    value: json!(format!("{key:?}")),
                },
                EventType::KeyRelease(key) => DeviceEvent {
                    kind: DeviceEventKind::KeyboardRelease,
                    value: json!(format!("{key:?}")),
                },
                _ => return,
            };

            let _ = app_handle.emit("device-changed", device_event);
        });

        IS_ENABLED.store(false, Ordering::SeqCst);
        LISTENER_STARTED.store(false, Ordering::SeqCst);

        if let Err(error) = result {
            eprintln!("failed to listen for input: {error:?}");
        }
    });
}

#[command]
pub fn stop_device_listening() {
    IS_ENABLED.store(false, Ordering::SeqCst);
}
