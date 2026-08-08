use gilrs::{EventType, Gilrs};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Runtime, command};

static IS_LISTENING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
pub enum GamepadEventKind {
    ButtonChanged,
    AxisChanged,
}

#[derive(Debug, Clone, Serialize)]
pub struct GamepadEvent {
    kind: GamepadEventKind,
    name: String,
    value: f32,
}

#[command]
pub async fn start_gamepad_listening<R: Runtime>(app_handle: AppHandle<R>) -> Result<(), String> {
    if IS_LISTENING.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(error) => {
            IS_LISTENING.store(false, Ordering::SeqCst);
            return Err(error.to_string());
        }
    };

    while IS_LISTENING.load(Ordering::SeqCst) {
        while let Some(event) = gilrs.next_event() {
            let gamepad_event = match event.event {
                EventType::ButtonChanged(button, value, ..) => GamepadEvent {
                    kind: GamepadEventKind::ButtonChanged,
                    name: format!("{button:?}"),
                    value,
                },
                EventType::AxisChanged(axis, value, ..) => GamepadEvent {
                    kind: GamepadEventKind::AxisChanged,
                    name: format!("{axis:?}"),
                    value,
                },
                _ => continue,
            };

            let _ = app_handle.emit("gamepad-changed", gamepad_event);
        }

        std::thread::sleep(std::time::Duration::from_millis(4));
    }

    Ok(())
}

#[command]
pub fn stop_gamepad_listening() {
    IS_LISTENING.store(false, Ordering::SeqCst);
}
