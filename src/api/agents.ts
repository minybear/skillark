import { invoke } from "@tauri-apps/api/core";
import type { AgentCandidate } from "../shared/agents";

export async function discoverAgents(
  manualSkillPaths: Record<string, string> = {},
): Promise<AgentCandidate[]> {
  return invoke<AgentCandidate[]>("discover_agents", {
    request: { manualSkillPaths },
  });
}

export async function cancelAgentDiscovery(): Promise<void> {
  return invoke("cancel_agent_discovery");
}
