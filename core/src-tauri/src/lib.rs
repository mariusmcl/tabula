use tauri::{Manager, State};
use tabula_blockchain::{AppState, Block};

#[tauri::command]
fn get_blockchain(state: State<AppState>) -> Result<Vec<Block>, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.get_chain())
}

#[tauri::command]
fn add_block(data: String, state: State<AppState>) -> Result<Vec<Block>, String> {
    let mut blockchain = state.blockchain.lock().unwrap();
    blockchain.add_block(data);
    Ok(blockchain.get_chain())
}

#[tauri::command]
fn validate_chain(state: State<AppState>) -> Result<bool, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.is_chain_valid())
}

#[tauri::command]
fn get_difficulty(state: State<AppState>) -> Result<usize, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.difficulty)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Tabula Blockchain!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::new(2);
            app.manage(state);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_blockchain,
            add_block,
            validate_chain,
            get_difficulty
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

