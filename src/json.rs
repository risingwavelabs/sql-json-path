pub trait Json {}

impl Json for &serde_json::Value {}
