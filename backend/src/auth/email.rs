use crate::{config::Config, error::AppError};
use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, AsyncSmtpTransport,
    AsyncTransport, Message, Tokio1Executor,
};
use rand::Rng;

pub fn generate_otp() -> String {
    format!("{:06}", rand::thread_rng().gen_range(0..=999999u32))
}

pub async fn send_otp(to_email: &str, otp: &str, config: &Config) -> crate::error::Result<()> {
    let smtp = config.smtp.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "SMTP not configured — cannot send email OTP"
        ))
    })?;

    let from: Mailbox = smtp
        .from
        .parse()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid SMTP from address: {e}")))?;
    let to: Mailbox = to_email
        .parse()
        .map_err(|_| AppError::BadRequest("invalid email address".to_string()))?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Cauldron account unlock code")
        .body(format!(
            "Your Cauldron unlock code is: {otp}\n\nThis code expires in 5 minutes.\n\nIf you did not request this, your account may be compromised."
        ))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("email build: {e}")))?;

    let creds = Credentials::new(smtp.username.clone(), smtp.password.clone());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp.host)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("SMTP relay setup: {e}")))?
        .credentials(creds)
        .build();

    mailer
        .send(email)
        .await
        .map(|_| ())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("SMTP send: {e}")))
}
