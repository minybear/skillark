use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WorkspaceId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceKind {
    Global,
    Project,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub kind: WorkspaceKind,
    pub name: String,
    pub root_path: Option<PathBuf>,
}
