mod adapters;
mod commands;
mod domain;
mod services;

use std::sync::Arc;

use adapters::event_sink::{CompositeEventSink, EventSink, TauriEventSink};
use adapters::filesystem::{map_config_io, AppPaths, RealConfigFilesystem};
use adapters::frp_admin::{FrpAdminAdapter, HealthProbe};
use adapters::sidecar::{SidecarAdapter, TauriSidecarAdapter};
use services::app_settings::AppSettingsStore;
use services::config_repository::ConfigRepository;
use services::config_transaction::ConfigTransactionService;
use services::diagnostics_service::DiagnosticsService;
use services::log_service::{FileLogSink, LogService};
use services::process_supervisor::{ProcessSupervisor, SupervisorTiming};
use services::shutdown_coordinator::ShutdownCoordinator;
use tauri::Manager;

pub struct AppServices {
    pub paths: AppPaths,
    pub config: Arc<ConfigRepository>,
    pub settings: Arc<AppSettingsStore>,
    pub logs: Arc<LogService>,
    pub processes: Arc<ProcessSupervisor>,
    pub transactions: Arc<ConfigTransactionService>,
    pub shutdown: Arc<ShutdownCoordinator>,
    pub frp_admin: Arc<FrpAdminAdapter>,
    pub diagnostics: Arc<DiagnosticsService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};

            let app_handle = app.handle().clone();
            let paths = AppPaths::from_app(&app_handle)?;
            std::fs::create_dir_all(&paths.config_dir).map_err(map_config_io)?;

            let filesystem = Arc::new(RealConfigFilesystem);
            let config = Arc::new(ConfigRepository::new(paths.clone(), filesystem));
            let settings = Arc::new(AppSettingsStore::load_or_default(&paths)?);
            let logs = Arc::new(LogService::new(paths.clone(), settings.clone()));
            let events: Arc<dyn EventSink> = Arc::new(CompositeEventSink::new(vec![
                Box::new(TauriEventSink::new(app_handle.clone())),
                Box::new(FileLogSink::new(logs.clone())),
            ]));
            let sidecar: Arc<dyn SidecarAdapter> =
                Arc::new(TauriSidecarAdapter::new(app_handle.clone()));
            let frp_admin = Arc::new(FrpAdminAdapter::new()?);
            let health: Arc<dyn HealthProbe> = frp_admin.clone();
            let processes = Arc::new(ProcessSupervisor::new(
                config.clone(),
                sidecar.clone(),
                health.clone(),
                events.clone(),
                SupervisorTiming::default(),
            ));
            let transactions = Arc::new(ConfigTransactionService::new(
                config.clone(),
                processes.clone(),
                events,
            ));
            let shutdown = Arc::new(ShutdownCoordinator::new(processes.clone()));
            let diagnostics = Arc::new(DiagnosticsService::new(
                paths.clone(),
                config.clone(),
                settings.clone(),
                sidecar,
                health,
                processes.clone(),
                env!("CARGO_PKG_VERSION"),
            ));
            app.manage(AppServices {
                paths,
                config,
                settings,
                logs,
                processes,
                transactions,
                shutdown,
                frp_admin,
                diagnostics,
            });

            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;
            let mut tray = tauri::tray::TrayIconBuilder::new()
                .tooltip("Frp Desktop Plus")
                .menu(&menu)
                .on_menu_event(
                    |app: &tauri::AppHandle, event: tauri::menu::MenuEvent| match event.id.as_ref()
                    {
                        "quit" => {
                            let app = app.clone();
                            let shutdown = app.state::<AppServices>().shutdown.clone();
                            tauri::async_runtime::spawn(async move {
                                if let Err(error) = shutdown.prepare().await {
                                    eprintln!("shutdown preparation failed: {error}");
                                }
                                app.exit(0);
                            });
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    },
                )
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config_snapshot,
            commands::config::validate_config_source,
            commands::config::preview_config_change,
            commands::config::apply_config_change,
            commands::config::restore_config_backup,
            commands::config::save_config_and_restart,
            commands::process::get_process_snapshot,
            commands::process::start_process,
            commands::process::stop_process,
            commands::process::restart_process,
            commands::process::stop_all_processes,
            commands::process::prepare_shutdown,
            commands::support::export_logs,
            commands::support::export_deploy_script,
            commands::support::get_frpc_traffic,
            commands::logs::delete_disk_logs,
            commands::settings::get_app_settings,
            commands::settings::update_app_settings,
            commands::settings::apply_local_monitor,
            commands::diagnostics::run_diagnostics,
            commands::diagnostics::export_diagnostics_pack,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            let shutdown = app_handle.state::<AppServices>().shutdown.clone();
            if !shutdown.is_completed() {
                api.prevent_exit();
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = shutdown.prepare().await {
                        eprintln!("shutdown preparation failed: {error}");
                    }
                    app_handle.exit(0);
                });
            }
        }
        tauri::RunEvent::Exit => {
            if !app_handle.state::<AppServices>().shutdown.is_completed() {
                eprintln!("application exited before shutdown preparation completed");
            }
        }
        _ => {}
    });
}
