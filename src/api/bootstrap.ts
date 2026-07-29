import { invoke } from "@tauri-apps/api/core";
import {
  browserFallbackStatus,
  type BootstrapStatus,
} from "../shared/bootstrap";

export async function loadBootstrapStatus(): Promise<BootstrapStatus> {
  try {
    return await invoke<BootstrapStatus>("get_bootstrap_status");
  } catch {
    return browserFallbackStatus;
  }
}
