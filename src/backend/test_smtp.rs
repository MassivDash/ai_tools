use lettre::{AsyncSmtpTransport, AsyncTransport, Message, transport::smtp::authentication::Credentials};
use lettre::Tokio1Executor;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smtp_server = "mail19.mydevil.net";
    let smtp_port = 587;
    let smtp_username = "rustwatcher@spaceout.pl";
    let smtp_password = "jCdN4j8zYG6c46g.nb0!F%_EJo5Ao.";

    let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());
    
    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_server)?
        .port(smtp_port)
        .credentials(creds)
        .build();

    let email = Message::builder()
        .from("rustwatcher@spaceout.pl".parse()?)
        .to("rustwatcher@spaceout.pl".parse()?)
        .subject("Test")
        .body("Test".to_string())?;

    match mailer.send(email).await {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
