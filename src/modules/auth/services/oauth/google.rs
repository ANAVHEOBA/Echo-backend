use anyhow::{anyhow, Result};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, RedirectUrl,
    TokenUrl, Scope, CsrfToken, PkceCodeChallenge,
    AuthorizationCode, PkceCodeVerifier,
};
use url::Url;
use crate::config::AppConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub given_name: String,
    pub family_name: String,
    pub picture: String,
}

// Type alias for a BasicClient with Auth and Token URLs set
type GoogleClient = oauth2::Client<
    oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
    oauth2::basic::BasicTokenResponse,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
    oauth2::EndpointSet,      // HasAuthUrl
    oauth2::EndpointNotSet,   // HasDeviceAuthUrl
    oauth2::EndpointNotSet,   // HasIntrospectionUrl
    oauth2::EndpointNotSet,   // HasRevocationUrl
    oauth2::EndpointSet,      // HasTokenUrl
>;

pub struct GoogleOAuthService {
    client: GoogleClient,
}

impl GoogleOAuthService {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let google_client_id = ClientId::new(config.google_client_id.clone());
        let google_client_secret = ClientSecret::new(config.google_client_secret().to_string());
        
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid authorization URL: {}", e))?;
            
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| anyhow!("Invalid token URL: {}", e))?;

        let callback_url = if config.host == "0.0.0.0" || config.host == "127.0.0.1" {
            format!("http://localhost:{}/api/auth/oauth/google/callback", config.port)
        } else {
             format!("https://echo-backend-t2q5.onrender.com/api/auth/oauth/google/callback")
        };

        let redirect_url = RedirectUrl::new(callback_url)
             .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;

        let client = oauth2::basic::BasicClient::new(google_client_id)
            .set_client_secret(google_client_secret)
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

        Ok(Self { client })
    }

    pub fn get_authorization_url(&self) -> (Url, CsrfToken, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token) = self.client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/gmail.readonly".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/gmail.send".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
            .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
            .add_scope(Scope::new("openid".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        (auth_url, csrf_token, pkce_verifier)
    }

    pub async fn exchange_code(&self, code: String, code_verifier: String) -> Result<oauth2::basic::BasicTokenResponse> {
        let pkce_verifier = PkceCodeVerifier::new(code_verifier);
        
        type GoogleErrorResponse = oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>;
        
        let token_result = self.client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&|req: oauth2::HttpRequest| async move {
                let (parts, body) = req.into_parts();
                let client = reqwest::Client::new();
                let mut rb = client.request(parts.method, parts.uri.to_string());
                for (name, value) in parts.headers {
                    if let Some(n) = name {
                        rb = rb.header(n, value);
                    }
                }
                rb = rb.body(body);
                
                let res = rb.send().await.map_err(|e| oauth2::RequestTokenError::<std::io::Error, GoogleErrorResponse>::Other(format!("Reqwest error: {}", e)))?;
                
                let mut builder = oauth2::http::Response::builder()
                    .status(res.status());
                    
                for (name, value) in res.headers() {
                    builder = builder.header(name, value);
                }
                
                let bytes = res.bytes().await.map_err(|e| oauth2::RequestTokenError::<std::io::Error, GoogleErrorResponse>::Other(format!("Body error: {}", e)))?;
                let body = bytes.to_vec();
                
                builder.body(body)
                    .map_err(|e| oauth2::RequestTokenError::<std::io::Error, GoogleErrorResponse>::Other(format!("HTTP error: {}", e)))
            })
            .await
            .map_err(|e| anyhow!("Token exchange failed: {}", e))?;
            
        Ok(token_result)
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<GoogleUserInfo> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://www.googleapis.com/oauth2/v2/userinfo")
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch user info: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("Google API returned error: {}", response.status()));
        }

        let user_info = response.json::<GoogleUserInfo>().await
            .map_err(|e| anyhow!("Failed to parse user info: {}", e))?;

        Ok(user_info)
    }
}
