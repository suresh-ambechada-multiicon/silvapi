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
    BinaryFile,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FormDataPartKind {
    Text,
    File,
}

impl Default for FormDataPartKind {
    fn default() -> Self {
        FormDataPartKind::Text
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormDataPart {
    pub id: String,
    pub name: String,
    pub value: String,
    pub enabled: bool,
    pub kind: FormDataPartKind,
    pub content_type: String,
}

impl FormDataPart {
    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            value: String::new(),
            enabled: true,
            kind: FormDataPartKind::Text,
            content_type: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RequestBody {
    pub body_type: BodyType,
    pub content: String,
    #[serde(default)]
    pub urlencoded: Vec<KeyValue>,
    #[serde(default)]
    pub form_data: Vec<FormDataPart>,
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
    AwsV4,
    Jwt,
    OAuth2,
}

impl Default for AuthType {
    fn default() -> Self {
        AuthType::None
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub auth_type: AuthType,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub bearer_token: String,
    #[serde(default = "default_bearer_prefix")]
    pub bearer_prefix: String,
    pub basic_username: String,
    pub basic_password: String,
    pub api_key_name: String,
    pub api_key_value: String,
    pub api_key_in_header: bool,
    #[serde(default)]
    pub aws_access_key_id: String,
    #[serde(default)]
    pub aws_secret_access_key: String,
    #[serde(default = "default_aws_service")]
    pub aws_service: String,
    #[serde(default = "default_aws_region")]
    pub aws_region: String,
    #[serde(default)]
    pub aws_session_token: String,
    #[serde(default = "default_jwt_algorithm")]
    pub jwt_algorithm: String,
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default)]
    pub jwt_secret_base64: bool,
    #[serde(default = "default_jwt_payload")]
    pub jwt_payload: String,
    #[serde(default = "default_oauth_grant_type")]
    pub oauth_grant_type: String,
    #[serde(default)]
    pub oauth_client_id: String,
    #[serde(default)]
    pub oauth_client_secret: String,
    #[serde(default = "default_oauth_authorization_url")]
    pub oauth_authorization_url: String,
    #[serde(default = "default_oauth_access_token_url")]
    pub oauth_access_token_url: String,
    #[serde(default)]
    pub oauth_redirect_uri: String,
    #[serde(default)]
    pub oauth_state: String,
    #[serde(default)]
    pub oauth_audience: String,
    #[serde(default = "default_oauth_token_target")]
    pub oauth_token_target: String,
    #[serde(default = "default_oauth_response_type")]
    pub oauth_response_type: String,
    #[serde(default)]
    pub oauth_use_pkce: bool,
    #[serde(default)]
    pub oauth_username: String,
    #[serde(default)]
    pub oauth_password: String,
    #[serde(default)]
    pub oauth_scope: String,
    #[serde(default = "default_auth_header_name")]
    pub oauth_header_name: String,
    #[serde(default = "default_bearer_prefix")]
    pub oauth_header_prefix: String,
    #[serde(default = "default_oauth_send_credentials")]
    pub oauth_send_credentials: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: AuthType::None,
            enabled: true,
            bearer_token: String::new(),
            bearer_prefix: default_bearer_prefix(),
            basic_username: String::new(),
            basic_password: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_in_header: true,
            aws_access_key_id: String::new(),
            aws_secret_access_key: String::new(),
            aws_service: default_aws_service(),
            aws_region: default_aws_region(),
            aws_session_token: String::new(),
            jwt_algorithm: default_jwt_algorithm(),
            jwt_secret: String::new(),
            jwt_secret_base64: false,
            jwt_payload: default_jwt_payload(),
            oauth_grant_type: default_oauth_grant_type(),
            oauth_client_id: String::new(),
            oauth_client_secret: String::new(),
            oauth_authorization_url: default_oauth_authorization_url(),
            oauth_access_token_url: default_oauth_access_token_url(),
            oauth_redirect_uri: String::new(),
            oauth_state: String::new(),
            oauth_audience: String::new(),
            oauth_token_target: default_oauth_token_target(),
            oauth_response_type: default_oauth_response_type(),
            oauth_use_pkce: false,
            oauth_username: String::new(),
            oauth_password: String::new(),
            oauth_scope: String::new(),
            oauth_header_name: default_auth_header_name(),
            oauth_header_prefix: default_bearer_prefix(),
            oauth_send_credentials: default_oauth_send_credentials(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_bearer_prefix() -> String {
    "Bearer".to_string()
}

fn default_aws_service() -> String {
    "sts".to_string()
}

fn default_aws_region() -> String {
    "us-east-1".to_string()
}

fn default_jwt_algorithm() -> String {
    "HS256".to_string()
}

fn default_jwt_payload() -> String {
    "{\n  \"foo\": \"bar\"\n}".to_string()
}

fn default_oauth_grant_type() -> String {
    "Authorization Code".to_string()
}

fn default_oauth_authorization_url() -> String {
    "https://github.com/login/oauth/authorize".to_string()
}

fn default_oauth_access_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_oauth_token_target() -> String {
    "access_token".to_string()
}

fn default_oauth_response_type() -> String {
    "Access Token".to_string()
}

fn default_auth_header_name() -> String {
    "Authorization".to_string()
}

fn default_oauth_send_credentials() -> String {
    "In Request Body".to_string()
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
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub variables: Vec<Variable>,
    pub items: Vec<CollectionItem>,
}

impl Folder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            headers: Vec::new(),
            auth: AuthConfig::default(),
            variables: Vec::new(),
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
        if fuzzy_match(self.name(), query) {
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
        if fuzzy_match(&self.name, query) {
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
                urlencoded: Vec::new(),
                form_data: Vec::new(),
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

/// Case-insensitive subsequence fuzzy match: returns true if all characters of
/// `needle` appear in `haystack` in order. An empty needle always matches.
pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut chars = haystack.chars().flat_map(|c| c.to_lowercase());
    for nc in needle.chars().flat_map(|c| c.to_lowercase()) {
        if !chars.any(|hc| hc == nc) {
            return false;
        }
    }
    true
}

#[cfg(test)]
#[path = "models_tests.rs"]
mod fuzzy_tests;

pub fn build_urlencoded_body(fields: &[KeyValue]) -> String {
    fields
        .iter()
        .filter(|field| field.enabled && !field.key.is_empty())
        .map(|field| format!("{}={}", urlencod(&field.key), urlencod(&field.value)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn urlencod(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
