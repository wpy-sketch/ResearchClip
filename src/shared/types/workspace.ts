/** IPC contract: keep in sync with src-tauri/src/domain/workspace.rs. */
export interface WorkspaceStatus {
  appName: string;
  appVersion: string;
  platform: string;
  dataDir: string;
  databasePath: string;
  schemaVersion: number;
  sqliteVersion: string;
}

export type RuntimeStatus =
  | { mode: 'desktop'; workspace: WorkspaceStatus }
  | { mode: 'preview' };
