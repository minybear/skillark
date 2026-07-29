use jsonschema::validator_for;
use serde_json::{json, Value};
use skillark_lib::commands::contracts::{
    AgentCandidateDto, ConflictKindDto, DeploymentPlanDto, DeploymentTargetDto, DetectionSignalDto,
    InstallModeDto, SkillFileDto, SkillManifestDto,
};

fn schema(source: &str) -> Value {
    serde_json::from_str(source).expect("schema must be valid JSON")
}

fn assert_valid(schema: &Value, instance: &Value) {
    let validator = validator_for(schema).expect("schema must compile");
    assert!(
        validator.is_valid(instance),
        "instance should match schema: {instance:#}"
    );
}

#[test]
fn agent_candidate_uses_frozen_external_field_names() {
    let candidate = AgentCandidateDto {
        agent_type: "codex".to_owned(),
        display_name: "Codex".to_owned(),
        confidence: 85,
        executable_path: Some(r"C:\tools\codex.exe".to_owned()),
        global_skill_path: Some(r"C:\Users\demo\.codex\skills".to_owned()),
        writable: Some(true),
        signals: vec![DetectionSignalDto {
            signal_type: "path_executable".to_owned(),
            matched: true,
            weight: 40,
            detail: None,
        }],
    };
    let instance = serde_json::to_value(candidate).expect("candidate must serialize");
    let contract = schema(include_str!(
        "../../docs/skillark/design/v0.1/contracts/agent-candidate.schema.json"
    ));

    assert_valid(&contract, &instance);
    assert_eq!(instance["agentType"], "codex");
    assert!(instance.get("kind").is_none());
}

#[test]
fn deployment_plan_requires_a_non_null_workspace_id() {
    let plan = DeploymentPlanDto {
        operation_id: "019b1d91-2970-7cc0-8d7a-3c933cfd6341".to_owned(),
        skill_version_id: "019b1d91-2970-7cc0-8d7a-3c933cfd6342".to_owned(),
        targets: vec![DeploymentTargetDto {
            agent_id: "codex-windows".to_owned(),
            workspace_id: "global-default".to_owned(),
            target_path: r"C:\Users\demo\.codex\skills\sample".to_owned(),
            mode: InstallModeDto::Copy,
            conflict: ConflictKindDto::None,
            warnings: vec![],
        }],
        requires_confirmation: false,
        warnings: vec![],
    };
    let instance = serde_json::to_value(plan).expect("plan must serialize");
    let contract = schema(include_str!(
        "../../docs/skillark/design/v0.1/contracts/deployment-plan.schema.json"
    ));

    assert_valid(&contract, &instance);
    assert_eq!(instance["targets"][0]["workspaceId"], "global-default");
}

#[test]
fn skill_manifest_matches_the_frozen_contract() {
    let hash = "0".repeat(64);
    let manifest = SkillManifestDto {
        name: "sample-skill".to_owned(),
        description: "A contract fixture".to_owned(),
        format: "agent-skills".to_owned(),
        files: vec![SkillFileDto {
            path: "SKILL.md".to_owned(),
            size: 42,
            sha256: hash.clone(),
        }],
        content_hash: hash,
        metadata: json!({}),
        warnings: vec![],
    };
    let instance = serde_json::to_value(manifest).expect("manifest must serialize");
    let contract = schema(include_str!(
        "../../docs/skillark/design/v0.1/contracts/skill-manifest.schema.json"
    ));

    assert_valid(&contract, &instance);
}

#[test]
fn legacy_kind_field_is_rejected() {
    let contract = schema(include_str!(
        "../../docs/skillark/design/v0.1/contracts/agent-candidate.schema.json"
    ));
    let validator = validator_for(&contract).expect("schema must compile");
    let legacy = json!({
        "kind": "codex",
        "displayName": "Codex",
        "confidence": 85,
        "signals": []
    });

    assert!(!validator.is_valid(&legacy));
}
