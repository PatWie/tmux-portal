use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tmux_interface::{
    KillSession, ListSessions, ListWindows, RenameWindow, SelectWindow, SwitchClient, Tmux,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxSession {
    pub name: String,
    pub windows: Vec<TmuxWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmuxWindow {
    pub id: String,
    pub name: String,
    pub session_name: String,
    pub active: bool,
}

pub fn get_current_session_name() -> Result<Option<String>> {
    use tmux_interface::{ListSessions, Tmux};

    // Try to get the current session name from tmux
    let output = Tmux::with_command(
        ListSessions::new().format("#{session_name}:#{?session_attached,attached,not_attached}"),
    )
    .output();

    match output {
        Ok(output) if output.status().success() => {
            let stdout_data = output.stdout();
            let stdout_str = String::from_utf8_lossy(&stdout_data);

            // Find the attached session
            for line in stdout_str.lines() {
                if line.ends_with(":attached") {
                    let session_name = line.trim_end_matches(":attached").to_string();
                    if !session_name.is_empty() {
                        return Ok(Some(session_name));
                    }
                }
            }

            Ok(None) // No attached session found
        }
        _ => Ok(None), // Not in a tmux session or tmux not available
    }
}

pub fn get_tmux_sessions() -> Result<Vec<TmuxSession>> {
    // Check if tmux server is running
    let sessions_output = match Tmux::with_command(ListSessions::new()).output() {
        Ok(output) => output,
        Err(_) => {
            // No tmux server running or tmux not available
            return Ok(vec![]);
        }
    };

    if !sessions_output.status().success() {
        return Ok(vec![]);
    }

    let stdout_data = sessions_output.stdout();
    let sessions_str = String::from_utf8_lossy(&stdout_data);
    let mut sessions = Vec::new();

    for line in sessions_str.lines() {
        if let Some(session_name) = parse_session_name(line) {
            let windows = get_session_windows(&session_name)?;
            sessions.push(TmuxSession {
                name: session_name,
                windows,
            });
        }
    }

    Ok(sessions)
}

fn parse_session_name(line: &str) -> Option<String> {
    // Parse session name from tmux list-sessions output
    // Format: "session_name: 1 windows (created ...)"
    line.find(':')
        .map(|colon_pos| line[..colon_pos].trim().to_string())
}

fn get_session_windows(session_name: &str) -> Result<Vec<TmuxWindow>> {
    let windows_output = Tmux::with_command(
        ListWindows::new()
            .target_session(session_name)
            .format("#{window_id}|#{window_name}|#{window_active}"),
    )
    .output()?;

    if !windows_output.status().success() {
        return Err(anyhow!(
            "Failed to get windows for session: {}",
            session_name
        ));
    }

    let stdout_data = windows_output.stdout();
    let windows_str = String::from_utf8(stdout_data)?;
    let mut windows = Vec::new();

    for line in windows_str.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() == 3 {
            windows.push(TmuxWindow {
                id: parts[0].to_string(),
                name: parts[1].to_string(),
                session_name: session_name.to_string(),
                active: parts[2] == "1",
            });
        }
    }

    Ok(windows)
}

pub fn switch_to_window(session_name: &str, window_id: &str) -> Result<()> {
    // First select the window using window ID for unique identification
    let select_output = Tmux::with_command(
        SelectWindow::new().target_window(format!("{session_name}:{window_id}")),
    )
    .output()?;

    if !select_output.status().success() {
        return Err(anyhow!(
            "Failed to select window: {}:{}",
            session_name,
            window_id
        ));
    }

    // Then switch to the session
    let switch_output =
        Tmux::with_command(SwitchClient::new().target_session(session_name)).output()?;

    if !switch_output.status().success() {
        return Err(anyhow!("Failed to switch to session: {}", session_name));
    }

    Ok(())
}

pub fn rename_window(session_name: &str, window_id: &str, new_name: &str) -> Result<()> {
    let output = Tmux::with_command(
        RenameWindow::new()
            .target_window(format!("{session_name}:{window_id}"))
            .new_name(new_name),
    )
    .output()?;

    if !output.status().success() {
        return Err(anyhow!(
            "Failed to rename window: {}:{}",
            session_name,
            window_id
        ));
    }

    Ok(())
}

pub fn delete_window(session_name: &str, window_id: &str) -> Result<()> {
    use tmux_interface::{KillWindow, Tmux};

    let target = format!("{session_name}:{window_id}");
    let output = Tmux::with_command(KillWindow::new().target_window(&target)).output()?;

    if !output.status().success() {
        let stderr = output.stderr();
        let error = String::from_utf8_lossy(&stderr);
        return Err(anyhow::anyhow!("Failed to delete window: {}", error));
    }

    Ok(())
}

pub fn switch_to_session(session_name: &str) -> Result<()> {
    let switch_output =
        Tmux::with_command(SwitchClient::new().target_session(session_name)).output()?;

    if !switch_output.status().success() {
        return Err(anyhow!("Failed to switch to session: {}", session_name));
    }

    Ok(())
}

pub fn kill_session(session_name: &str) -> Result<()> {
    let output = Tmux::with_command(KillSession::new().target_session(session_name)).output()?;

    if !output.status().success() {
        return Err(anyhow!("Failed to kill session '{}'", session_name));
    }

    Ok(())
}

pub fn rename_session(old_name: &str, new_name: &str) -> Result<()> {
    use tmux_interface::{RenameSession, Tmux};

    let output = Tmux::with_command(
        RenameSession::new()
            .target_session(old_name)
            .new_name(new_name),
    )
    .output()?;

    if !output.status().success() {
        let stderr = output.stderr();
        let error = String::from_utf8_lossy(&stderr);
        return Err(anyhow!(
            "Failed to rename session '{}' to '{}': {}",
            old_name,
            new_name,
            error
        ));
    }

    Ok(())
}
pub fn find_window_in_session(session_name: &str, window_name: &str) -> Result<Option<TmuxWindow>> {
    // First check if the session exists
    let session_exists = Tmux::with_command(ListSessions::new())
        .output()
        .map(|output| {
            let stdout_data = output.stdout();
            let sessions_str = String::from_utf8_lossy(&stdout_data);
            sessions_str.lines().any(|line| {
                if let Some(name) = parse_session_name(line) {
                    name == session_name
                } else {
                    false
                }
            })
        })
        .unwrap_or(false);

    if !session_exists {
        return Ok(None);
    }

    // Session exists, now look for the window
    let windows = get_session_windows(session_name)?;

    // Find the window with the matching name
    let window = windows.into_iter().find(|w| w.name == window_name);

    Ok(window)
}

pub fn create_new_window(session_name: &str) -> Result<()> {
    use tmux_interface::{NewWindow, Tmux};

    // Use the -d flag to create the window without attaching to it
    let output = Tmux::with_command(
        NewWindow::new()
            .detached() // -d flag
            .target_window(session_name),
    )
    .output()?;

    if !output.status().success() {
        let stderr = output.stderr();
        let error = String::from_utf8_lossy(&stderr);
        return Err(anyhow::anyhow!("Failed to create new window: {}", error));
    }

    Ok(())
}

pub fn swap_windows_in_tmux(session_name: &str, window1_id: &str, window2_id: &str) -> Result<()> {
    use tmux_interface::{ListWindows, SelectWindow, SwapWindow, Tmux};

    // First, check which window is currently active
    let active_window_output = Tmux::with_command(
        ListWindows::new()
            .target_session(session_name)
            .format("#{window_id}:#{window_active}"),
    )
    .output()?;

    let mut active_window_id = String::new();
    if active_window_output.status().success() {
        let stdout_data = active_window_output.stdout();
        let stdout_str = String::from_utf8_lossy(&stdout_data);

        // Find the active window
        for line in stdout_str.lines() {
            if line.ends_with(":1") {
                // Active window has window_active=1
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    active_window_id = parts[0].to_string();
                    break;
                }
            }
        }
    }

    // Perform the swap
    let output = Tmux::with_command(
        SwapWindow::new()
            .src_window(window1_id)
            .dst_window(window2_id),
    )
    .output()?;

    if !output.status().success() {
        let stderr = output.stderr();
        let error = String::from_utf8_lossy(&stderr);
        return Err(anyhow::anyhow!("Failed to swap windows: {}", error));
    }

    // If one of the swapped windows was active, make sure it stays active
    if !active_window_id.is_empty()
        && (active_window_id == window1_id || active_window_id == window2_id)
    {
        let select_output =
            Tmux::with_command(SelectWindow::new().target_window(&active_window_id)).output()?;

        if !select_output.status().success() {
            // Don't fail the whole operation if select-window fails
            // The swap was successful, just the active window tracking might be off
        }
    }

    Ok(())
}

pub fn switch_to_session_and_window(
    session_name: &str,
    window_name: &str,
    path: &std::path::Path,
) -> Result<()> {
    use tmux_interface::{HasSession, NewSession, NewWindow, SwitchClient, Tmux};

    // Try to find the window in the session
    match find_window_in_session(session_name, window_name)? {
        Some(window) => {
            // Window exists, switch to it
            switch_to_window(&window.session_name, &window.id)
        }
        None => {
            // Window doesn't exist, check if session exists
            let session_exists = Tmux::with_command(HasSession::new().target_session(session_name))
                .output()
                .map(|output| output.status().success())
                .unwrap_or(false);

            if session_exists {
                // Create new window in existing session
                let path_str = path.to_str().unwrap_or("");
                let output = Tmux::with_command(
                    NewWindow::new()
                        .target_window(session_name)
                        .window_name(window_name)
                        .start_directory(path_str)
                        .select(), // -S flag
                )
                .output()?;

                if !output.status().success() {
                    let stderr = output.stderr();
                    let error = String::from_utf8_lossy(&stderr);
                    return Err(anyhow::anyhow!("Failed to create window: {}", error));
                }
            } else {
                // Create new session with window
                let path_str = path.to_str().unwrap_or("");
                let output = Tmux::with_command(
                    NewSession::new()
                        .detached() // -d flag
                        .session_name(session_name)
                        .window_name(window_name)
                        .start_directory(path_str),
                )
                .output()?;

                if !output.status().success() {
                    let stderr = output.stderr();
                    let error = String::from_utf8_lossy(&stderr);
                    return Err(anyhow::anyhow!("Failed to create session: {}", error));
                }

                // Switch to the session:window
                let session_window_name = format!("{session_name}:{window_name}");
                let switch_output =
                    Tmux::with_command(SwitchClient::new().target_session(&session_window_name))
                        .output()?;

                if !switch_output.status().success() {
                    let stderr = switch_output.stderr();
                    let error = String::from_utf8_lossy(&stderr);
                    return Err(anyhow::anyhow!(
                        "Failed to switch to session:window: {}",
                        error
                    ));
                }
            }

            Ok(())
        }
    }
}
