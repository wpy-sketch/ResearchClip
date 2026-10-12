export interface ApplicationInfo {
  mode: 'desktop' | 'preview';
  version: string;
}

/** IPC contract: src-tauri/src/domain/application.rs. */
export interface ConnectionTestResponse {
  message: string;
  appVersion: string;
}
