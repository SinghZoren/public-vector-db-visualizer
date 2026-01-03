use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = tursoExecute, catch)]
    async fn turso_execute(url: String, token: String, sql: String, args: JsValue) -> Result<JsValue, JsValue>;
}

#[allow(dead_code)]
#[derive(Serialize)]
struct TursoRequest {
    #[serde(rename = "type")]
    request_type: String,
    stmt: Option<Stmt>,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct Stmt {
    sql: String,
    args: Vec<serde_json::Value>,
}

#[allow(dead_code)]
pub struct TursoClient {
    url: String,
    token: String,
}

impl TursoClient {
    #[allow(dead_code)]
    pub fn new(url: String, token: String) -> Self {
        let clean_url = if url.contains("v2/pipeline") {
            url
        } else {
            url.trim_end_matches('/').to_string() + "/v2/pipeline"
        };
        
        Self {
            url: clean_url.replace("https://corsproxy.io/?", ""),
            token,
        }
    }

    #[allow(dead_code)]
    pub async fn execute_sql(&self, sql: &str, args: Vec<serde_json::Value>) -> Result<Vec<Vec<serde_json::Value>>, Box<dyn std::error::Error>> {
        let args_js = serde_wasm_bindgen::to_value(&args)?;
        
        match turso_execute(self.url.clone(), self.token.clone(), sql.to_string(), args_js).await {
            Ok(rows_js) => {
                let rows: Vec<Vec<serde_json::Value>> = serde_wasm_bindgen::from_value(rows_js)?;
                Ok(rows)
            }
            Err(e) => Err(format!("JS fetch error: {:?}", e).into()),
        }
    }
}
