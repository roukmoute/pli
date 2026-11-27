use std::{env, time::Duration};

use async_imap::Client;
use tokio::{net::TcpStream, time::timeout};
use webmail_server::Error;

#[tokio::test]
#[ignore = "requiert le jeu base du banc GreenMail"]
async fn greenmail_alice_retrouve_les_vingt_messages_de_base() -> Result<(), Error> {
    let port: u16 = env::var("PLI_BENCH_IMAP_PORT")?.parse()?;
    timeout(Duration::from_secs(10), async {
        let stream = TcpStream::connect(("127.0.0.1", port)).await?;
        let mut client = Client::new(stream);
        client
            .read_response()
            .await?
            .ok_or("Bannière IMAP absente")?;
        let mut session = client
            .login("alice@recette.test", "alice-test")
            .await
            .map_err(|(error, _)| error)?;

        assert_eq!(session.examine("INBOX").await?.exists, 20);
        session.logout().await?;
        Ok(())
    })
    .await?
}
