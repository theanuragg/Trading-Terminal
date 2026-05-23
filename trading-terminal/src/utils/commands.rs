use crate::app::App;

// simplistic parser implementing a few Bloomberg-style commands for meme coins
// expand as needed.

pub fn parse_and_dispatch(app: &mut App, cmd: &str) {
    let lower = cmd.to_lowercase();

    // some triggers
    if lower == "cancel" {
        app.add_log("<CANCEL> command received".to_string());
        return;
    }
    if lower == "go" {
        app.add_log("<GO> command received".to_string());
        return;
    }

    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    // Example: "pepe sol" -> select token
    if parts.len() == 2 {
        let symbol = parts[0].to_uppercase();
        let chain = parts[1].to_uppercase();
        app.add_log(format!("lookup {} on {}...", symbol, chain));
        // TODO: trigger fetch same as home selection
        return;
    }

    if lower.starts_with("hype ") {
        let token = &cmd[5..];
        app.add_log(format!("Showing hype meter for {}", token));
        return;
    }

    if lower.starts_with("rug ") {
        let token = &cmd[4..];
        app.add_log(format!("Running rug-check on {}", token));
        return;
    }

    if lower.starts_with("ca:") {
        // could open by contract address
        app.add_log(format!("Fetching token by contract {}", cmd));
        return;
    }

    if lower == "port" {
        app.add_log("Opening portfolio panel".to_string());
        app.add_panel("PORT", crate::app::PanelContent::Home); // placeholder
        return;
    }

    // fallback
    app.add_log(format!("Unknown command: {}", cmd));
}
