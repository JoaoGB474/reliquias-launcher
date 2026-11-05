#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]
mod bandeja;
mod comandos;
mod relatorio;
use tauri::Manager;
fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(comandos::AppState::default())
        .setup(|app| {
            bandeja::instalar(app.handle())?;
            bandeja::vigiar_launcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if bandeja::fechar(window.app_handle()) {
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            comandos::estado,
            comandos::entrar,
            comandos::sair,
            comandos::jogar,
            comandos::cancelar,
            comandos::verificar_arquivos,
            comandos::ajustes,
            comandos::salvar_ajustes,
            comandos::abrir_pasta,
            comandos::gerar_relatorio,
            comandos::status,
            comandos::novidades,
            comandos::sugerir_pasta,
            comandos::escolher_pasta,
            comandos::validar_pasta,
            comandos::definir_pasta
        ])
        .build(tauri::generate_context!());
    let app = match result {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Falha ao abrir o launcher: {e}");
            std::process::exit(1);
        }
    };
    app.run(|app, event| {
        if let tauri::RunEvent::Exit = event {
            let pronto = app
                .state::<comandos::AppState>()
                .atualizacao
                .lock()
                .ok()
                .and_then(|mut p| p.take());
            if let Some(pronto) = pronto {
                tauri_plugin_single_instance::destroy(app);
                if pronto.aplicar().is_err() {
                    eprintln!("Não foi possível aplicar a atualização do launcher");
                }
            }
        }
    });
}
