use meralus_shared::{IncomingPacket, OutgoingPacket, Player, ServerConnection};
use std::{error::Error, sync::Arc};
use tokio::{net::TcpListener, sync::RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server = TcpListener::bind("192.168.1.5:37565").await?;

    println!("Server listening on {}", server.local_addr()?);

    let players = Arc::new(RwLock::new(Vec::new()));

    loop {
        let (socket, addr) = server.accept().await?;

        println!("Accepted connection from {addr}");

        let players = players.clone();

        tokio::spawn(async move {
            let mut connection = ServerConnection::new(socket);
            let mut current_player_name = String::new();

            while let Some(packet) = connection.receive().await {
                match packet {
                    Ok(IncomingPacket::PlayerConnected { name }) => {
                        current_player_name.clone_from(&name);

                        players.write().await.push(Player {
                            nickname: name,
                            position: glam::Vec3::ZERO,
                        });
                    }
                    Ok(IncomingPacket::PlayerMoved { .. }) => todo!(),
                    Ok(IncomingPacket::GetPlayers) => connection
                        .send(OutgoingPacket::PlayersList {
                            players: players.read().await.clone(),
                        })
                        .await
                        .unwrap(),
                    Err(err) => println!("{err}"),
                }
            }

            println!("Closed connection from {addr}");

            players
                .write()
                .await
                .retain(|player| player.nickname != current_player_name);
        });
    }
}

#[cfg(test)]
mod tests {
    use async_compression::tokio::write::{ZlibDecoder, ZlibEncoder};
    use glam::IVec2;
    use meralus_world::Chunk;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_chunk_compressing() {
        let mut chunk = Chunk::new(IVec2::new(0, 0));

        chunk.generate_surface(0);

        let serialized = chunk.serialize();
        let mut compressed = Vec::new();

        let mut encoder = ZlibEncoder::new(&mut compressed);

        encoder.write_all(&serialized).await.unwrap();
        encoder.shutdown().await.unwrap();

        println!(
            "Serialized: {} bytes. Compressed: {} bytes.",
            serialized.len(),
            compressed.len()
        );

        let mut data = Vec::new();
        let mut decoder = ZlibDecoder::new(&mut data);

        decoder.write_all(&compressed).await.unwrap();
        decoder.shutdown().await.unwrap();

        let deserialized = Chunk::deserialize(&data).unwrap();

        assert_eq!(chunk.origin, deserialized.origin);
        // assert_eq!(chunk.blocks, deserialized.blocks);
        // assert_eq!(chunk.light_levels, deserialized.light_levels);
    }
}
