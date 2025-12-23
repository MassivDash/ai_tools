use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ChromaDB,
    WebsiteCheck,
}

fn main() {
    let t = ToolType::ChromaDB;
    let json = serde_json::to_string(&t).unwrap();
    println!("Serialized ChromaDB: {}", json);
}
