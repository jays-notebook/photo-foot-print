//! Native confirmation with distinct explicit-save and dismissal results.

use rfd::{AsyncMessageDialog, MessageButtons, MessageDialogResult, MessageLevel};

use crate::error::WireError;

fn explicit_save_selected(result: MessageDialogResult) -> bool {
    matches!(result, MessageDialogResult::Custom(label) if label == "Save")
}

#[tauri::command]
pub async fn confirm_save(
    window: tauri::Window,
    title: String,
    message: String,
) -> Result<bool, WireError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let parent = window.clone();
    window
        .run_on_main_thread(move || {
            // Construct the native future on the main thread, including the
            // parent handle lookup required by AppKit. Await it off that thread.
            // Cancel is the first/default button on both platforms; AppKit also
            // assigns Escape to the button titled "Cancel". Windows dismissal
            // returns raw Cancel, distinct from the custom Save button.
            // Do not use tauri-plugin-dialog's show_with_result: it maps raw
            // Cancel to the second custom label, losing this safety distinction.
            let dialog = AsyncMessageDialog::new()
                .set_parent(&parent)
                .set_title(title)
                .set_description(message)
                .set_level(MessageLevel::Warning)
                .set_buttons(MessageButtons::OkCancelCustom(
                    "Cancel".into(),
                    "Save".into(),
                ))
                .show();
            tauri::async_runtime::spawn(async move {
                let _ = tx.send(explicit_save_selected(dialog.await));
            });
        })
        .map_err(|e| WireError::Io {
            detail: format!("show save confirmation: {e}"),
        })?;

    rx.await.map_err(|e| WireError::Io {
        detail: format!("save confirmation channel: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_save_approves_the_write() {
        assert!(explicit_save_selected(MessageDialogResult::Custom(
            "Save".into()
        )));

        for result in [
            MessageDialogResult::Cancel,
            MessageDialogResult::Custom("Cancel".into()),
            MessageDialogResult::Ok,
            MessageDialogResult::Yes,
            MessageDialogResult::No,
            MessageDialogResult::Custom(String::new()),
            MessageDialogResult::Custom("Unexpected button".into()),
        ] {
            assert!(
                !explicit_save_selected(result.clone()),
                "unexpected approval: {result:?}"
            );
        }
    }
}
