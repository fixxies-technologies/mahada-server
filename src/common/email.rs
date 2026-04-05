use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const RESEND_API_URL: &str = "https://api.resend.com/emails";

#[derive(Debug, Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendResponse {
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct EmailClient {
    api_key: String,
    from: String,
    http: Client,
}

pub struct EmailTemplate(String);

impl EmailTemplate {
    pub fn new(html: impl Into<String>) -> Self {
        Self(html.into())
    }

    pub fn render(self, vars: HashMap<&str, String>) -> String {
        let mut html = self.0;
        for (key, value) in vars {
            html = html.replace(&format!("{{{{{}}}}}", key), &value);
        }
        html
    }
}

pub struct Templates;

impl Templates {
    pub fn otp() -> EmailTemplate {
        EmailTemplate::new(include_str!("../templates/email/otp.html"))
    }

    pub fn invitation() -> EmailTemplate {
        EmailTemplate::new(include_str!("../templates/email/invitation.html"))
    }

    pub fn email_verification() -> EmailTemplate {
        EmailTemplate::new(include_str!("../templates/email/verification.html"))
    }
}

impl EmailClient {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            api_key,
            from,
            http: Client::new(),
        }
    }

    pub fn from_env() -> Self {
        let api_key = std::env::var("RESEND_API_KEY").expect("RESEND_API_KEY must be set");
        let from = std::env::var("RESEND_FROM")
            .unwrap_or_else(|_| "Mahada <noreply@mahada.co>".to_string());
        Self::new(api_key, from)
    }

    pub async fn send(&self, to: &str, subject: &str, html: String) -> Result<ResendResponse> {
        let res = self
            .http
            .post(RESEND_API_URL)
            .bearer_auth(&self.api_key)
            .json(&ResendRequest {
                from: self.from.clone(),
                to: vec![to.to_string()],
                subject: subject.to_string(),
                html,
            })
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Resend error {}: {}", status, body));
        }

        Ok(res.json::<ResendResponse>().await?)
    }

    pub async fn send_template(
        &self,
        to: &str,
        subject: &str,
        template: EmailTemplate,
        vars: HashMap<&str, String>,
    ) -> Result<ResendResponse> {
        let html = template.render(vars);
        self.send(to, subject, html).await
    }

    pub async fn send_otp(&self, to: &str, otp: &str) -> Result<ResendResponse> {
        let mut vars = HashMap::new();
        vars.insert("otp", otp.to_string());

        self.send_template(
            to,
            "Your Verification Code - Mahada",
            Templates::otp(),
            vars,
        )
        .await
    }

    pub async fn send_verification(
        &self,
        to: &str,
        token: &str,
        base_url: &str,
    ) -> Result<ResendResponse> {
        let url = format!("{}/verify?token={}", base_url, token);
        let mut vars = HashMap::new();
        vars.insert("verification_url", url);

        self.send_template(
            to,
            "Verify your email address",
            Templates::email_verification(),
            vars,
        )
        .await
    }

    pub async fn send_invitation(
        &self,
        to: &str,
        data: InvitationEmailData<'_>,
    ) -> Result<ResendResponse> {
        let is_existing = data.pin_code == "EXISTING_USER";

        let body_text = if is_existing {
            format!("{} wants to connect with you on Mahada.", data.inviter_name)
        } else {
            format!(
                "{} has invited you to join Mahada, a research community platform.",
                data.inviter_name
            )
        };

        let custom_message_html = data.custom_message.map(|m| {
            format!(
                r#"<div style="background:#fafafa;padding:16px;border-left:3px solid #000;margin:20px 0;">
                    <p style="margin:0;font-style:italic;color:#000;font-size:14px;">&ldquo;{}&rdquo;</p>
                </div>"#,
                m
            )
        }).unwrap_or_default();

        let cta_text = if is_existing {
            "View on Mahada"
        } else {
            "Accept Invitation"
        };
        let subject = if is_existing {
            format!("{} wants to connect on Mahada", data.inviter_name)
        } else {
            format!("{} invited you to join Mahada", data.inviter_name)
        };

        let mut vars = HashMap::new();
        vars.insert("inviter_name", data.inviter_name.to_string());
        vars.insert(
            "inviter_avatar",
            data.inviter_avatar.unwrap_or("").to_string(),
        );
        vars.insert(
            "inviter_title",
            data.inviter_title.unwrap_or("Researcher").to_string(),
        );
        vars.insert(
            "inviter_institution",
            data.inviter_institution.unwrap_or("").to_string(),
        );
        vars.insert("body_text", body_text);
        vars.insert("custom_message_html", custom_message_html);
        vars.insert("cta_url", data.join_url.to_string());
        vars.insert("cta_text", cta_text.to_string());

        self.send_template(to, &subject, Templates::invitation(), vars)
            .await
    }
}

// ── Template engine ───────────────────────────────────────────────────────────

/// A template is just an HTML string with `{{variable}}` placeholders.
/// Call `EmailTemplate::render(vars)` to produce the final HTML.

pub struct InvitationEmailData<'a> {
    pub inviter_name: &'a str,
    pub inviter_avatar: Option<&'a str>,
    pub inviter_title: Option<&'a str>,
    pub inviter_institution: Option<&'a str>,
    pub custom_message: Option<&'a str>,
    pub pin_code: &'a str,
    pub join_url: &'a str,
}
