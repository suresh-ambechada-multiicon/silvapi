use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
    OPTIONS,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
        }
    }

    pub fn all() -> Vec<HttpMethod> {
        vec![
            HttpMethod::GET,
            HttpMethod::POST,
            HttpMethod::PUT,
            HttpMethod::PATCH,
            HttpMethod::DELETE,
            HttpMethod::HEAD,
            HttpMethod::OPTIONS,
        ]
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BodyType {
    None,
    Json,
    FormData,
    UrlEncoded,
    Raw,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RequestBody {
    pub body_type: BodyType,
    pub content: String,
}

impl Default for BodyType {
    fn default() -> Self {
        BodyType::None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthType {
    None,
    Bearer,
    Basic,
    ApiKey,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    pub bearer_token: String,
    pub basic_username: String,
    pub basic_password: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_in_header: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub id: String,
    pub key: String,
    pub value: String,
    pub enabled: bool,
    pub description: String,
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            key: key.into(),
            value: value.into(),
            enabled: true,
            description: String::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new("", "")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub params: Vec<KeyValue>,
    pub headers: Vec<KeyValue>,
    pub body: RequestBody,
    pub auth: AuthConfig,
    pub description: String,
}

impl Default for ApiRequest {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "New Request".to_string(),
            method: HttpMethod::GET,
            url: String::new(),
            params: vec![KeyValue::empty()],
            headers: vec![
                KeyValue::new("Content-Type", "application/json"),
                KeyValue::new("Accept", "application/json"),
            ],
            body: RequestBody::default(),
            auth: AuthConfig::default(),
            description: String::new(),
        }
    }
}

impl ApiRequest {
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub items: Vec<CollectionItem>,
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CollectionItem {
    Folder(Folder),
    Request(ApiRequest),
}

impl CollectionItem {
    pub fn id(&self) -> &str {
        match self {
            CollectionItem::Folder(f) => &f.id,
            CollectionItem::Request(r) => &r.id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            CollectionItem::Folder(f) => &f.name,
            CollectionItem::Request(r) => &r.name,
        }
    }

    pub fn is_folder(&self) -> bool {
        matches!(self, CollectionItem::Folder(_))
    }

    pub fn matches_query(&self, query: &str) -> bool {
        if self.name().to_lowercase().contains(query) {
            return true;
        }
        if let CollectionItem::Folder(f) = self {
            return f.items.iter().any(|item| item.matches_query(query));
        }
        false
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub items: Vec<CollectionItem>,
    pub variables: Vec<Variable>,
}

impl Collection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            items: Vec::new(),
            variables: Vec::new(),
        }
    }

    pub fn matches_query(&self, query: &str) -> bool {
        if self.name.to_lowercase().contains(query) {
            return true;
        }
        self.items.iter().any(|item| item.matches_query(query))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    pub id: String,
    pub name: String,
    pub value: String,
    pub enabled: bool,
}

impl Variable {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub collections: Vec<Collection>,
    pub variables: Vec<Variable>,
    #[serde(default, skip_serializing)]
    pub response_cache: std::collections::HashMap<String, Vec<HttpResponse>>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            collections: Vec::new(),
            variables: Vec::new(),
            response_cache: std::collections::HashMap::new(),
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        let mut ws = Self::new("My Workspace");

        let mut col = Collection::new("Sample API");
        let mut folder = Folder::new("Users");
        folder.items.push(CollectionItem::Request(ApiRequest {
            name: "List users".into(),
            method: HttpMethod::GET,
            url: "https://jsonplaceholder.typicode.com/users".into(),
            ..Default::default()
        }));
        folder.items.push(CollectionItem::Request(ApiRequest {
            name: "Get user".into(),
            method: HttpMethod::GET,
            url: "https://jsonplaceholder.typicode.com/users/1".into(),
            ..Default::default()
        }));
        folder.items.push(CollectionItem::Request(ApiRequest {
            name: "Create user".into(),
            method: HttpMethod::POST,
            url: "https://jsonplaceholder.typicode.com/users".into(),
            body: RequestBody {
                body_type: BodyType::Json,
                content: "{\n  \"name\": \"John Doe\",\n  \"email\": \"john@example.com\"\n}"
                    .into(),
            },
            ..Default::default()
        }));
        col.items.push(CollectionItem::Folder(folder));
        col.items.push(CollectionItem::Request(ApiRequest {
            name: "Health check".into(),
            method: HttpMethod::GET,
            url: "https://jsonplaceholder.typicode.com/todos/1".into(),
            ..Default::default()
        }));
        ws.collections.push(col);

        ws.variables.push(Variable::new(
            "baseUrl",
            "https://jsonplaceholder.typicode.com",
        ));
        ws.variables.push(Variable::new("userId", "1"));
        ws
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TimelineIcon {
    Setting,
    Request,
    Info,
    Response,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub name: String,
    pub timestamp: String,
    pub icon: TimelineIcon,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time_ms: u64,
    pub size_bytes: usize,
    pub timeline: Vec<TimelineEvent>,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    pub fn is_error(&self) -> bool {
        self.status >= 400
    }

    pub fn formatted_size(&self) -> String {
        if self.size_bytes < 1024 {
            format!("{} B", self.size_bytes)
        } else if self.size_bytes < 1024 * 1024 {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.size_bytes as f64 / (1024.0 * 1024.0))
        }
    }
}
