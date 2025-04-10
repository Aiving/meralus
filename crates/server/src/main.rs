use std::error::Error;

use async_compression::tokio::bufread::ZlibDecoder;
use tokio::{
    io::{AsyncReadExt, BufReader, Interest},
    net::TcpListener,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server = TcpListener::bind("192.168.1.5:37565").await?;

    println!("Server listening on {}", server.local_addr()?);

    loop {
        let (socket, addr) = server.accept().await?;

        println!("Accepted connection from {addr}");

        let ready = socket
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;

        tokio::spawn(async move {
            // Handle the connection
            let mut buf = [0; 1024];

            if ready.is_readable() {
                // let mut socket = ZlibDecoder::new(BufReader::new(socket));

                match socket.try_read(&mut buf) {
                    Ok(n) => {
                        let mut decoder = ZlibDecoder::new(BufReader::new(&buf[..]));
                        let mut buf = Vec::new();

                        decoder
                            .read_to_end(&mut buf)
                            .await
                            .expect("Failed to decode zlib data");

                        println!(
                            "Received {} (compressed: {n}) bytes: {:?}",
                            buf.len(),
                            str::from_utf8(&buf)
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to read from socket: {e}");
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use async_compression::tokio::write::{ZlibDecoder, ZlibEncoder};
    use glam::IVec2;
    use meralus_meshing::Chunk;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_chunk_compressing() {
        let chunk = Chunk::from_perlin_noise(IVec2::new(0, 0), 0);
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
        assert_eq!(chunk.blocks, deserialized.blocks);
        assert_eq!(chunk.light_levels, deserialized.light_levels);
    }
}
