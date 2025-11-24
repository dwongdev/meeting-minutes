// Tauri commands for built-in AI model management
// Exposes model download, status, and management functionality to frontend

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::sync::Mutex;

use super::model_manager::{ModelInfo, ModelManager};

// ============================================================================
// Global State
// ============================================================================

/// Global model manager instance
pub struct ModelManagerState(pub Arc<Mutex<Option<ModelManager>>>);

/// Initialize the model manager
pub async fn init_model_manager<R: Runtime>(app: &AppHandle<R>) -> anyhow::Result<()> {
    let models_dir = app.path().app_data_dir()?.join("models").join("summary");

    let manager = ModelManager::new_with_models_dir(Some(models_dir))?;
    manager.init().await?;

    let state: State<ModelManagerState> = app.state();
    let mut manager_lock = state.0.lock().await;
    *manager_lock = Some(manager);

    log::info!("Built-in AI model manager initialized");
    Ok(())
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// List all available built-in AI models with their status
#[tauri::command]
pub async fn builtin_ai_list_models<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
) -> Result<Vec<ModelInfo>, String> {
    // Ensure manager is initialized
    {
        let manager_lock = state.0.lock().await;
        if manager_lock.is_none() {
            drop(manager_lock);
            init_model_manager(&app)
                .await
                .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
        }
    }

    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    let models = manager.list_models().await;
    Ok(models)
}

/// Get information about a specific model
#[tauri::command]
pub async fn builtin_ai_get_model_info<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<Option<ModelInfo>, String> {
    // Ensure manager is initialized
    {
        let manager_lock = state.0.lock().await;
        if manager_lock.is_none() {
            drop(manager_lock);
            init_model_manager(&app)
                .await
                .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
        }
    }

    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    let info = manager.get_model_info(&model_name).await;
    Ok(info)
}

/// Download a built-in AI model with progress updates
#[tauri::command]
pub async fn builtin_ai_download_model<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    // Ensure manager is initialized
    {
        let manager_lock = state.0.lock().await;
        if manager_lock.is_none() {
            drop(manager_lock);
            init_model_manager(&app)
                .await
                .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
        }
    }

    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    // Create progress callback that emits Tauri events
    let app_clone = app.clone();
    let model_name_clone = model_name.clone();
    let progress_callback = Box::new(move |progress: u8| {
        let _ = app_clone.emit(
            "builtin-ai-download-progress",
            serde_json::json!({
                "model": model_name_clone,
                "progress": progress,
                "status": if progress == 100 { "completed" } else { "downloading" }
            }),
        );
    });

    manager
        .download_model(&model_name, Some(progress_callback))
        .await
        .map_err(|e| e.to_string())?;

    // Emit completion event
    let _ = app.emit(
        "builtin-ai-download-progress",
        serde_json::json!({
            "model": model_name,
            "progress": 100,
            "status": "completed"
        }),
    );

    Ok(())
}

/// Cancel an ongoing model download
#[tauri::command]
pub async fn builtin_ai_cancel_download(
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    manager
        .cancel_download(&model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Delete a corrupted or available model file
#[tauri::command]
pub async fn builtin_ai_delete_model(
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<(), String> {
    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    manager
        .delete_model(&model_name)
        .await
        .map_err(|e| e.to_string())
}

/// Check if a model is ready to use
#[tauri::command]
pub async fn builtin_ai_is_model_ready<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, ModelManagerState>,
    model_name: String,
) -> Result<bool, String> {
    // Ensure manager is initialized
    {
        let manager_lock = state.0.lock().await;
        if manager_lock.is_none() {
            drop(manager_lock);
            init_model_manager(&app)
                .await
                .map_err(|e| format!("Failed to initialize model manager: {}", e))?;
        }
    }

    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    let ready = manager.is_model_ready(&model_name).await;
    Ok(ready)
}

/// Get the models directory path
#[tauri::command]
pub async fn builtin_ai_get_models_directory(
    state: State<'_, ModelManagerState>,
) -> Result<String, String> {
    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    let dir = manager.get_models_directory();
    Ok(dir.to_string_lossy().to_string())
}

/// Open the models folder in system file explorer
#[tauri::command]
pub async fn builtin_ai_open_models_folder(
    state: State<'_, ModelManagerState>,
) -> Result<(), String> {
    let manager_lock = state.0.lock().await;
    let manager = manager_lock
        .as_ref()
        .ok_or_else(|| "Model manager not initialized".to_string())?;

    let models_dir = manager.get_models_directory();

    // Create directory if it doesn't exist
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir)
            .map_err(|e| format!("Failed to create models directory: {}", e))?;
    }

    let folder_path = models_dir.to_string_lossy().to_string();

    // Open in system file explorer using platform-specific commands
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&folder_path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    log::info!("Opened models folder: {}", folder_path);
    Ok(())
}
