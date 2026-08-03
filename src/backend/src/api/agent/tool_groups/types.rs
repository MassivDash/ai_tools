use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolGroup {
    pub id: i64,
    pub name: String,
    pub tool_types: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateToolGroupRequest {
    pub name: String,
    pub tool_types: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateToolGroupRequest {
    pub name: String,
    pub tool_types: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolGroupsResponse {
    pub groups: Vec<ToolGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToolGroupResponse {
    pub group: ToolGroup,
}
