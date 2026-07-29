import { invoke } from "@tauri-apps/api/core";

export type AgentOverride = {
  agentType: string;
  displayName: string;
  cliName?: string | null;
  configDir?: string | null;
  skillDir?: string | null;
  skillPathOverride?: string | null;
  isCustom: boolean;
};

export async function getAgentOverrides(): Promise<AgentOverride[]> {
  try {
    return await invoke<AgentOverride[]>("get_agent_overrides");
  } catch {
    return [];
  }
}

export async function saveAgentOverride(o: AgentOverride): Promise<void> {
  await invoke("save_agent_override", { request: o });
}

export async function deleteAgentOverride(agentType: string): Promise<void> {
  await invoke("delete_agent_override", { agentType });
}
