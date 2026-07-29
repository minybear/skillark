export type DetectionSignal = {
  type: string;
  matched: boolean;
  weight: number;
  detail: string | null;
};

export type AgentCandidate = {
  agentType: string;
  displayName: string;
  confidence: number;
  executablePath: string | null;
  globalSkillPath: string | null;
  writable: boolean | null;
  signals: DetectionSignal[];
};

export type DiscoveryState = "idle" | "scanning" | "ready" | "error";
