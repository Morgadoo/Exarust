//! The original's modal message box, shown through the desktop's own dialog
//! tool.
//!
//! The original's logger opens a top-most, task-modal box for its level-6
//! (error icon) and level-7 (warning icon) messages and waits for it to be
//! closed ([`exa_game::frame::MessageBox`]). Winit has no message box, so
//! Exarust asks the desktop's dialog tool for one (`zenity`, else
//! `kdialog`) and waits for it the same way; without either, or in an
//! unattended run, the box's caption and text go to standard error.
//!
//! The one level-6 box Exarust can meet is the original's "minimum
//! requirements" box, opened when the graphics device lacks what the
//! renderer needs: [`install_gpu_failure_box`] shows it when the renderer
//! finds no usable graphics adapter.

use std::process::{Command, Stdio};

use exa_game::frame::{
    CRITICAL_FAILURE_LEVEL, MINIMUM_REQUIREMENTS_TEXT, MessageBox, MessageBoxIcon,
};

/// What Bevy's renderer says when no adapter suits it.
const NO_ADAPTER: &str = "Unable to find a GPU";

/// Shows a message box and waits until it is closed. `interactive` false
/// (an unattended run) writes it to standard error instead, so no run
/// blocks on a dialog nobody sees.
pub fn show(message: &MessageBox, interactive: bool) {
    if interactive && (zenity(message) || kdialog(message)) {
        return;
    }
    let kind = match message.icon {
        MessageBoxIcon::Error => "error",
        MessageBoxIcon::Warning => "warning",
    };
    eprintln!("{} ({kind}): {}", message.caption, message.text);
}

/// The box through `zenity`; false when the tool cannot be started.
fn zenity(message: &MessageBox) -> bool {
    let kind = match message.icon {
        MessageBoxIcon::Error => "--error",
        MessageBoxIcon::Warning => "--warning",
    };
    Command::new("zenity")
        .args([kind, "--no-markup", "--title", &message.caption])
        .args(["--text", &message.text])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// The box through `kdialog`; false when the tool cannot be started.
fn kdialog(message: &MessageBox) -> bool {
    let kind = match message.icon {
        MessageBoxIcon::Error => "--error",
        MessageBoxIcon::Warning => "--sorry",
    };
    Command::new("kdialog")
        .args([kind, &message.text, "--title", &message.caption])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Whether a panic's message is the renderer's "no usable adapter".
pub fn is_gpu_failure(panic_message: &str) -> bool {
    panic_message.contains(NO_ADAPTER)
}

/// The box the original opens for an unusable graphics device: level 6
/// (the error icon), captioned with the window's title.
pub fn gpu_failure_box(window_title: &str) -> MessageBox {
    MessageBox::for_level(
        CRITICAL_FAILURE_LEVEL,
        window_title,
        MINIMUM_REQUIREMENTS_TEXT,
    )
    .unwrap_or_else(|| MessageBox::from_kind(2, window_title, MINIMUM_REQUIREMENTS_TEXT))
}

/// Installs a panic hook that, after the usual report, opens the minimum
/// requirements box when the renderer found no usable adapter.
pub fn install_gpu_failure_box(window_title: &'static str, interactive: bool) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let payload = info.payload();
        let text = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        if is_gpu_failure(&text) {
            show(&gpu_failure_box(window_title), interactive);
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_no_adapter_panic_opens_the_minimum_requirements_box() {
        assert!(is_gpu_failure(
            "Unable to find a GPU! Make sure you have installed required drivers!"
        ));
        assert!(!is_gpu_failure("index out of bounds"));
        let message = gpu_failure_box("Exarust");
        assert_eq!(message.caption, "Exarust");
        assert_eq!(message.icon, MessageBoxIcon::Error);
        assert_eq!(message.style(), 0x42010);
        assert_eq!(message.text, MINIMUM_REQUIREMENTS_TEXT);
    }
}
